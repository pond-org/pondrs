//! Pipeline graph validation.

use std::prelude::v1::*;
use std::collections::HashSet;
use std::fmt;

use super::types::PipelineGraph;

#[derive(Debug)]
pub enum ValidationError {
    /// A node requires a dataset that no other node produces and is not a source dataset.
    MissingInput {
        node_name: &'static str,
        dataset_name: String,
        dataset_id: usize,
    },
    /// A dataset is produced by multiple nodes.
    DuplicateOutput {
        dataset_name: String,
        dataset_id: usize,
        producer_names: Vec<&'static str>,
    },
    /// A pipeline declares an input that none of its children consume.
    UnusedPipelineInput {
        pipeline_name: &'static str,
        dataset_name: String,
        dataset_id: usize,
    },
    /// A pipeline declares an output that none of its children produce.
    UnproducedPipelineOutput {
        pipeline_name: &'static str,
        dataset_name: String,
        dataset_id: usize,
    },
    /// A param dataset appears as an output of a node.
    ParamWritten {
        node_name: &'static str,
        dataset_name: String,
        dataset_id: usize,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::MissingInput { node_name, dataset_name, .. } => {
                write!(f, "Node '{node_name}' requires dataset '{dataset_name}', but it is not produced by any node")
            }
            ValidationError::DuplicateOutput { dataset_name, producer_names, .. } => {
                let names = producer_names.join("', '");
                write!(f, "Dataset '{dataset_name}' is produced by multiple nodes: '{names}'")
            }
            ValidationError::UnusedPipelineInput { pipeline_name, dataset_name, .. } => {
                write!(f, "Pipeline '{pipeline_name}' declares input '{dataset_name}', but none of its children consume it")
            }
            ValidationError::UnproducedPipelineOutput { pipeline_name, dataset_name, .. } => {
                write!(f, "Pipeline '{pipeline_name}' declares output '{dataset_name}', but none of its children produce it")
            }
            ValidationError::ParamWritten { node_name, dataset_name, .. } => {
                write!(f, "Node '{node_name}' writes to param dataset '{dataset_name}', but params are read-only")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl PipelineGraph<'_> {
    /// Validate the pipeline graph, returning all errors found.
    pub fn check(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        self.check_missing_inputs(&mut errors);
        self.check_duplicate_outputs(&mut errors);
        self.check_param_writes(&mut errors);
        self.check_pipeline_contracts(&mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn dataset_name(&self, id: usize) -> String {
        self.dataset_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("<unknown:{id:#x}>"))
    }

    /// Check that every leaf node's non-param inputs are produced by another node.
    fn check_missing_inputs(&self, errors: &mut Vec<ValidationError>) {
        let produced: HashSet<usize> = self.nodes
            .iter()
            .filter(|n| !n.is_pipe)
            .flat_map(|n| n.outputs.iter().map(|d| d.id))
            .collect();

        for node in self.nodes.iter().filter(|n| !n.is_pipe) {
            for input in &node.inputs {
                if !input.is_param && !produced.contains(&input.id) {
                    errors.push(ValidationError::MissingInput {
                        node_name: node.name,
                        dataset_name: self.dataset_name(input.id),
                        dataset_id: input.id,
                    });
                }
            }
        }
    }

    /// Check that no dataset is produced by more than one node.
    fn check_duplicate_outputs(&self, errors: &mut Vec<ValidationError>) {
        let mut producers: std::collections::HashMap<usize, Vec<&'static str>> =
            std::collections::HashMap::new();

        for node in self.nodes.iter().filter(|n| !n.is_pipe) {
            for output in &node.outputs {
                producers.entry(output.id).or_default().push(node.name);
            }
        }

        for (dataset_id, names) in producers {
            if names.len() > 1 {
                errors.push(ValidationError::DuplicateOutput {
                    dataset_name: self.dataset_name(dataset_id),
                    dataset_id,
                    producer_names: names,
                });
            }
        }
    }

    /// Check that no node writes to a param dataset.
    fn check_param_writes(&self, errors: &mut Vec<ValidationError>) {
        for node in self.nodes.iter().filter(|n| !n.is_pipe) {
            for output in &node.outputs {
                if output.is_param {
                    errors.push(ValidationError::ParamWritten {
                        node_name: node.name,
                        dataset_name: self.dataset_name(output.id),
                        dataset_id: output.id,
                    });
                }
            }
        }
    }

    /// Check that pipeline declared inputs/outputs match what children actually consume/produce.
    fn check_pipeline_contracts(&self, errors: &mut Vec<ValidationError>) {
        for node in self.nodes.iter().filter(|n| n.is_pipe) {
            // Collect all dataset IDs consumed/produced by descendant leaves
            let mut child_inputs: HashSet<usize> = HashSet::new();
            let mut child_outputs: HashSet<usize> = HashSet::new();
            self.collect_descendant_datasets(node, &mut child_inputs, &mut child_outputs);

            // Check declared inputs are consumed by children
            for input in &node.inputs {
                if !child_inputs.contains(&input.id) {
                    errors.push(ValidationError::UnusedPipelineInput {
                        pipeline_name: node.name,
                        dataset_name: self.dataset_name(input.id),
                        dataset_id: input.id,
                    });
                }
            }

            // Check declared outputs are produced by children
            for output in &node.outputs {
                if !child_outputs.contains(&output.id) {
                    errors.push(ValidationError::UnproducedPipelineOutput {
                        pipeline_name: node.name,
                        dataset_name: self.dataset_name(output.id),
                        dataset_id: output.id,
                    });
                }
            }
        }
    }

    fn collect_descendant_datasets(
        &self,
        node: &super::types::GraphNode,
        inputs: &mut HashSet<usize>,
        outputs: &mut HashSet<usize>,
    ) {
        for &child_idx in &node.pipe_children {
            let child = &self.nodes[child_idx];
            if !child.is_pipe {
                inputs.extend(child.inputs.iter().map(|d| d.id));
                outputs.extend(child.outputs.iter().map(|d| d.id));
            } else {
                self.collect_descendant_datasets(child, inputs, outputs);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datasets::{MemoryDataset, Param};
    use crate::core::{Node, Pipeline};
    use crate::graph::build_pipeline_graph;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestCatalog {
        a: MemoryDataset<i32>,
        b: MemoryDataset<i32>,
        c: MemoryDataset<i32>,
    }

    #[derive(Serialize)]
    struct TestParams {
        initial_value: Param<i32>,
    }

    #[test]
    fn test_valid_pipeline_passes() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams { initial_value: Param(1) };
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.initial_value,), output: (&catalog.a,) },
            Node { name: "n2", func: |v| (v,), input: (&catalog.a,), output: (&catalog.b,) },
        );
        let graph = build_pipeline_graph(&pipe, &catalog, &params);
        assert!(graph.check().is_ok());
    }

    #[test]
    fn test_missing_input_detected() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams { initial_value: Param(1) };
        // n1 reads from catalog.a, but nothing produces it and it's not a param
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&catalog.a,), output: (&catalog.b,) },
        );
        let graph = build_pipeline_graph(&pipe, &catalog, &params);
        let errs = graph.check().unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(matches!(&errs[0], ValidationError::MissingInput { node_name: "n1", .. }));
    }

    #[test]
    fn test_duplicate_output_detected() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams { initial_value: Param(1) };
        // Both n1 and n2 write to catalog.a
        let pipe = (
            Node { name: "n1", func: |v| (v,), input: (&params.initial_value,), output: (&catalog.a,) },
            Node { name: "n2", func: |v| (v,), input: (&params.initial_value,), output: (&catalog.a,) },
        );
        let graph = build_pipeline_graph(&pipe, &catalog, &params);
        let errs = graph.check().unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, ValidationError::DuplicateOutput { .. })));
    }

    #[test]
    fn test_unproduced_pipeline_output_detected() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams { initial_value: Param(1) };
        // Pipeline declares output c, but children only produce b
        let pipe = (
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&catalog.a,), output: (&catalog.b,) },
                ),
                input: (&catalog.a,),
                output: (&catalog.c,),  // c is never produced!
            },
        );
        let graph = build_pipeline_graph(&pipe, &catalog, &params);
        let errs = graph.check().unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, ValidationError::UnproducedPipelineOutput {
            pipeline_name: "inner", ..
        })));
    }

    #[test]
    fn test_unused_pipeline_input_detected() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams { initial_value: Param(1) };
        // Pipeline declares input a, but children only read from b
        let pipe = (
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&catalog.b,), output: (&catalog.c,) },
                ),
                input: (&catalog.a,),  // a is never consumed!
                output: (&catalog.c,),
            },
        );
        let graph = build_pipeline_graph(&pipe, &catalog, &params);
        let errs = graph.check().unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, ValidationError::UnusedPipelineInput {
            pipeline_name: "inner", ..
        })));
    }

    #[test]
    fn test_valid_nested_pipeline_passes() {
        let catalog = TestCatalog {
            a: MemoryDataset::new(),
            b: MemoryDataset::new(),
            c: MemoryDataset::new(),
        };
        let params = TestParams { initial_value: Param(1) };
        let pipe = (
            Node { name: "n0", func: |v| (v,), input: (&params.initial_value,), output: (&catalog.a,) },
            Pipeline {
                name: "inner",
                steps: (
                    Node { name: "n1", func: |v| (v,), input: (&catalog.a,), output: (&catalog.b,) },
                    Node { name: "n2", func: |v| (v,), input: (&catalog.b,), output: (&catalog.c,) },
                ),
                input: (&catalog.a,),
                output: (&catalog.c,),
            },
        );
        let graph = build_pipeline_graph(&pipe, &catalog, &params);
        assert!(graph.check().is_ok());
    }
}
