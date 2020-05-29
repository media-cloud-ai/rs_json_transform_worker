use jq_rs::JqProgram;
use jxon::{json_to_xml, xml_to_json};
use mcai_worker_sdk::{debug, info, job::*, McaiChannel, MessageError, ParametersContainer};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;
use xmltree::Element;

pub fn process(
  _channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let result = match job
    .get_string_parameter("template_mode")
    .unwrap_or_else(|| "string".to_string())
    .as_str()
  {
    "string" => jq_process(&job, false),
    "file" => jq_process(&job, true),
    mode => Err(Error::new(
      ErrorKind::Other,
      format!("mode {:?} not supported.", mode,),
    )),
  };

  result
    .map(|_| job_result.clone().with_status(JobStatus::Completed))
    .map_err(|error| MessageError::from(error, job_result))
}

fn jq_process(job: &Job, is_source_path_template: bool) -> Result<(), Error> {
  let mut program = get_filter_program(job, is_source_path_template)?;
  let source_paths = get_source_paths(job)?;
  let destination_path = get_destination_path(job)?;
  let output_mode = job
    .get_string_parameter("output_mode")
    .unwrap_or_else(|| "json".to_string());

  for source_path in source_paths {
    let input_path = Path::new(&source_path);
    let output_path = Path::new(&destination_path);

    info!("{}", format!("Run jq program on '{}'.", source_path));
    if !input_path.is_file() {
      return Err(Error::new(
        ErrorKind::Other,
        format!("No such file: {:?}", source_path),
      ));
    }

    info!("{}", format!("Parse content of '{}'.", source_path));
    let file_content = read_source_content(input_path)?;

    debug!("Run jq program.");
    let output_content = &program.run(&file_content.to_string()).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!("Unable to process with JQ: {}", e.to_string()),
      )
    })?;

    debug!("Write jq program result.");
    info!("{}", output_mode);
    write_destination_content(output_path, output_content, &output_mode)?;
  }

  Ok(())
}

fn get_filter_program(job: &Job, is_source_path_template: bool) -> Result<JqProgram, Error> {
  let template_source = job
    .get_string_parameter("template")
    .ok_or_else(|| Error::new(ErrorKind::Other, "Missing template parameter"))?;

  let template_content = if is_source_path_template {
    fs::read_to_string(template_source)?
  } else {
    template_source
  };

  info!("Compile template as a jq program");
  let program =
    jq_rs::compile(&template_content).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
  Ok(program)
}

fn get_source_paths(job: &Job) -> Result<Vec<String>, Error> {
  let source_paths = if let Some(source_path) = job.get_string_parameter("source_path") {
    vec![source_path]
  } else {
    job
      .get_array_of_strings_parameter("source_paths")
      .ok_or_else(|| Error::new(ErrorKind::Other, "Source path(s) must be defined."))?
  };

  Ok(source_paths)
}

fn get_destination_path(job: &Job) -> Result<String, Error> {
  let destination_path = job
    .get_string_parameter("destination_path")
    .ok_or_else(|| Error::new(ErrorKind::Other, "Destination path must be defined."))?;

  Ok(destination_path)
}

fn read_source_content(path: &Path) -> Result<String, Error> {
  let raw_content = fs::read_to_string(path).map_err(|e| {
    Error::new(
      ErrorKind::Other,
      format!(
        "Unable to read file {}: {}",
        path.to_string_lossy(),
        e.to_string()
      ),
    )
  })?;

  let content = if Element::parse(raw_content.as_bytes()).is_ok() {
    let json = xml_to_json(&raw_content).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!(
          "Unable to convert input to json {}: {}",
          path.to_string_lossy(),
          e.to_string()
        ),
      )
    })?;

    serde_json::to_string(&json)?
  } else {
    raw_content
  };

  Ok(content)
}

fn write_destination_content(path: &Path, content: &str, mode: &str) -> Result<(), Error> {
  let transformed_content = if mode == "xml" {
    json_to_xml(content, None).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!("Unable to write xml from json: {}", e.to_string()),
      )
    })?
  } else {
    content.to_owned()
  };

  fs::write(path, &transformed_content).map_err(|e| {
    Error::new(
      ErrorKind::Other,
      format!("Unable to write generated result: {}", e.to_string()),
    )
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn process_with_string_test_ok() {
    let message = fs::read_to_string("examples/json_mapping.json").unwrap();

    let job = Job::new(&message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    println!("{:?}", result);
    assert!(result.is_ok());

    assert_eq!(
      fs::read_to_string("tests/destination_1.json").unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn process_with_template_file_test_ok() {
    let message = fs::read_to_string("examples/xml_to_json.json").unwrap();

    let job = Job::new(&message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    assert!(result.is_ok());

    assert_eq!(
      fs::read_to_string("tests/destination_2.json").unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn process_xml_to_xml_ok() {
    let message = fs::read_to_string("examples/xml_to_xml.json").unwrap();

    let job = Job::new(&message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    println!("{:?}", result);
    assert!(result.is_ok());

    assert_eq!(
      fs::read_to_string("tests/destination_3.xml").unwrap(),
      fs::read_to_string("tests/source_3.xml").unwrap()
    );
  }

  #[test]
  fn process_xml_to_json_ok() {
    let message = fs::read_to_string("examples/xml_to_json.json").unwrap();

    let job = Job::new(&message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    println!("{:?}", result);
    assert!(result.is_ok());

    assert_eq!(
      fs::read_to_string("tests/destination_4.json").unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn process_json_to_xml_ok() {
    let message = fs::read_to_string("examples/json_to_xml.json").unwrap();

    let job = Job::new(&message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    assert!(result.is_ok());

    assert_eq!(
      fs::read_to_string("tests/destination_5.xml").unwrap(),
      r#"<name type="str">John Doe</name>"#
    );
  }

  #[test]
  fn process_test_error() {
    let message = r#"{
      "parameters": [
        {
          "id": "source_paths",
          "type": "array_of_strings",
          "value": [
            "tests/missing_source.json"
          ]
        },
        {
          "id": "template_mode",
          "type": "string",
          "value": "string"
        },
        {
          "id": "template",
          "type": "string",
          "value": ".name"
        },
        {
          "id": "destination_path",
          "type": "string",
          "value": "tests/destination_6.json"
        }
      ],
      "job_id": 124
    }"#;

    let job = Job::new(message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    let job_result = JobResult::new(124)
      .with_status(JobStatus::Error)
      .with_message(r#"IO Error: No such file: "tests/missing_source.json""#);

    assert_eq!(result, Err(MessageError::ProcessingError(job_result)));
  }

  #[test]
  fn mode_test_error() {
    let message = r#"{
      "parameters": [
        {
          "id": "source_paths",
          "type": "array_of_strings",
          "value": [
            "tests/source_7.json"
          ]
        },
        {
          "id": "template_mode",
          "type": "string",
          "value": "url"
        }
      ],
      "job_id": 0
    }"#;

    let job = Job::new(message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    let job_result = JobResult::new(0)
      .with_status(JobStatus::Error)
      .with_message(r#"IO Error: mode "url" not supported."#);

    assert_eq!(result, Err(MessageError::ProcessingError(job_result)));
  }
}
