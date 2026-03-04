use quick_xml::Reader;
use quick_xml::events::Event;

/// A raw test result parsed from JUnit XML or libtest JSON.
#[derive(Debug, Clone)]
pub struct RawTestResult {
    pub name: String,
    pub classname: Option<String>,
    pub status: RawTestStatus,
    pub duration_secs: Option<f64>,
    pub message: Option<String>,
}

/// Raw status before mapping to SpecForge's TestStatus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawTestStatus {
    Pass,
    Fail,
    Skip,
    Error,
}

/// Parse JUnit XML content into raw test results.
pub fn parse_junit_xml(xml: &str) -> Result<Vec<RawTestResult>, String> {
    let mut reader = Reader::from_str(xml);
    let mut results = Vec::new();

    let mut current_name: Option<String> = None;
    let mut current_classname: Option<String> = None;
    let mut current_time: Option<f64> = None;
    let mut current_status = RawTestStatus::Pass;
    let mut current_message: Option<String> = None;
    let mut in_testcase = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                match e.local_name().as_ref() {
                    b"testcase" => {
                        in_testcase = true;
                        current_status = RawTestStatus::Pass;
                        current_message = None;
                        current_name = None;
                        current_classname = None;
                        current_time = None;

                        read_testcase_attrs(e, &mut current_name, &mut current_classname, &mut current_time);
                    }
                    b"failure" if in_testcase => {
                        current_status = RawTestStatus::Fail;
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"message" {
                                current_message = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    b"error" if in_testcase => {
                        current_status = RawTestStatus::Error;
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"message" {
                                current_message = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    b"skipped" if in_testcase => {
                        current_status = RawTestStatus::Skip;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                match e.local_name().as_ref() {
                    b"testcase" => {
                        // Self-closing <testcase ... /> — always a pass
                        let mut name = None;
                        let mut classname = None;
                        let mut time = None;
                        read_testcase_attrs(e, &mut name, &mut classname, &mut time);
                        if let Some(n) = name {
                            results.push(RawTestResult {
                                name: n,
                                classname,
                                status: RawTestStatus::Pass,
                                duration_secs: time,
                                message: None,
                            });
                        }
                    }
                    b"failure" if in_testcase => {
                        current_status = RawTestStatus::Fail;
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"message" {
                                current_message = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    b"error" if in_testcase => {
                        current_status = RawTestStatus::Error;
                        for attr in e.attributes().flatten() {
                            if attr.key.local_name().as_ref() == b"message" {
                                current_message = Some(
                                    String::from_utf8_lossy(&attr.value).to_string(),
                                );
                            }
                        }
                    }
                    b"skipped" if in_testcase => {
                        current_status = RawTestStatus::Skip;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if e.local_name().as_ref() == b"testcase" && in_testcase {
                    if let Some(name) = current_name.take() {
                        results.push(RawTestResult {
                            name,
                            classname: current_classname.take(),
                            status: current_status,
                            duration_secs: current_time,
                            message: current_message.take(),
                        });
                    }
                    in_testcase = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
    }

    Ok(results)
}

fn read_testcase_attrs(
    e: &quick_xml::events::BytesStart<'_>,
    name: &mut Option<String>,
    classname: &mut Option<String>,
    time: &mut Option<f64>,
) {
    for attr in e.attributes().flatten() {
        match attr.key.local_name().as_ref() {
            b"name" => {
                *name = Some(String::from_utf8_lossy(&attr.value).to_string());
            }
            b"classname" => {
                let val = String::from_utf8_lossy(&attr.value).to_string();
                if !val.is_empty() {
                    *classname = Some(val);
                }
            }
            b"time" => {
                *time = String::from_utf8_lossy(&attr.value).parse::<f64>().ok();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_passing_tests() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="tests" tests="2">
    <testcase name="validate_input__rejects_empty" classname="tests::validate_input" time="0.001">
    </testcase>
    <testcase name="validate_input__accepts_valid" classname="tests::validate_input" time="0.002">
    </testcase>
  </testsuite>
</testsuites>"#;
        let results = parse_junit_xml(xml).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "validate_input__rejects_empty");
        assert_eq!(results[0].status, RawTestStatus::Pass);
        assert_eq!(results[1].name, "validate_input__accepts_valid");
    }

    #[test]
    fn parse_failures() {
        let xml = r#"<testsuites>
  <testsuite name="tests" tests="1">
    <testcase name="data_integrity__check" time="0.005">
      <failure message="assertion failed">expected true, got false</failure>
    </testcase>
  </testsuite>
</testsuites>"#;
        let results = parse_junit_xml(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, RawTestStatus::Fail);
        assert_eq!(results[0].message.as_deref(), Some("assertion failed"));
    }

    #[test]
    fn parse_skipped() {
        let xml = r#"<testsuites>
  <testsuite>
    <testcase name="some_test" time="0.0">
      <skipped/>
    </testcase>
  </testsuite>
</testsuites>"#;
        let results = parse_junit_xml(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, RawTestStatus::Skip);
    }

    #[test]
    fn parse_empty_input() {
        let xml = r#"<testsuites></testsuites>"#;
        let results = parse_junit_xml(xml).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn parse_duration() {
        let xml = r#"<testsuites>
  <testsuite>
    <testcase name="fast_test" time="0.042">
    </testcase>
  </testsuite>
</testsuites>"#;
        let results = parse_junit_xml(xml).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].duration_secs, Some(0.042));
    }

    #[test]
    fn malformed_xml_returns_error() {
        let xml = "<testsuites><testsuite><not closed";
        let result = parse_junit_xml(xml);
        assert!(result.is_err());
    }
}
