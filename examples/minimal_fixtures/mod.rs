//! Shared fixture helpers for the minimal-family examples.

use std::path::Path;

pub fn write_readings_csv(dir: &Path) {
    std::fs::write(
        dir.join("readings.csv"),
        "\
name,value
a,0.8
b,0.3
c,0.6
d,0.9
",
    )
    .unwrap();
}

pub fn write_catalog_yml(dir: &Path) {
    std::fs::write(
        dir.join("catalog.yml"),
        format!(
            "\
readings:
  path: {d}/readings.csv
  separator: \",\"
summary: {{}}
report:
  path: {d}/report.json
",
            d = dir.display()
        ),
    )
    .unwrap();
}
