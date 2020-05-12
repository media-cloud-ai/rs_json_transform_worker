use jq_rs::JqProgram;
use mcai_worker_sdk::{debug, info, job::*, McaiChannel, MessageError, ParametersContainer};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

pub fn process(
  _channel: Option<McaiChannel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  let result = match job
    .get_string_parameter("mode")
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

  for source_path in source_paths {
    let input_path = Path::new(&source_path);

    info!("{}", format!("Run jq program on '{}'.", source_path));
    if !Path::new(&source_path).is_file() {
      return Err(Error::new(
        ErrorKind::Other,
        format!("No such file: {:?}", source_path),
      ));
    }

    let file_content = fs::read_to_string(input_path).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!("Unable to read file {}: {}", source_path, e.to_string()),
      )
    })?;

    debug!("Run jq program.");
    let output_content = &program.run(&file_content.to_string()).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!("Unable to process with JQ: {}", e.to_string()),
      )
    })?;

    debug!("Write jq program result.");
    fs::write(destination_path.clone(), output_content).map_err(|e| {
      Error::new(
        ErrorKind::Other,
        format!("Unable to write generated result: {}", e.to_string()),
      )
    })?;
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn process_with_string_test_ok() {
    let content = r#"{
      "name": "John Doe",
      "age": 43,
      "phones": [
          "+44 1234567",
          "+44 2345678"
      ]
    }"#;

    fs::write("/tmp/source.json", content).unwrap();

    let message = r#"{
      "parameters": [
        {
          "id": "source_paths",
          "type": "array_of_strings",
          "value": [
            "/tmp/source.json"
          ]
        },
        {
          "id": "template",
          "type": "string",
          "value": ".name"
        },
        {
          "id": "destination_path",
          "type": "string",
          "value": "/tmp/destination.json"
        }
      ],
      "job_id": 123
    }"#;

    let job = Job::new(message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    println!("{:?}", result);
    assert!(result.is_ok());

    let destination_path = Path::new("/tmp/destination.json");
    assert!(destination_path.exists());
    assert_eq!(
      fs::read_to_string(&destination_path).unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn process_with_template_url_test_ok() {
    let content = r#"{
      "name": "John Doe",
      "age": 43,
      "phones": [
          "+44 1234567",
          "+44 2345678"
      ]
    }"#;

    fs::write("/tmp/source.json", content).unwrap();
    fs::write("/tmp/template.jq", ".name").unwrap();

    let message = r#"{
      "parameters": [
        {
          "id": "source_paths",
          "type": "array_of_strings",
          "value": [
            "/tmp/source.json"
          ]
        },
        {
          "id": "mode",
          "type": "string",
          "value": "file"
        },
        {
          "id": "template",
          "type": "string",
          "value": "/tmp/template.jq"
        },
        {
          "id": "destination_path",
          "type": "string",
          "value": "/tmp/destination.json"
        }
      ],
      "job_id": 123
    }"#;

    let job = Job::new(message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    assert!(result.is_ok());

    let destination_path = Path::new("/tmp/destination.json");
    assert!(destination_path.exists());
    assert_eq!(
      fs::read_to_string(&destination_path).unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn process_test_error() {
    let content = r#"{
      "name": "John Doe",
      "age": 43,
      "phones": [
          "+44 1234567",
          "+44 2345678"
      ]
    }"#;

    fs::write("/tmp/source_1.json", content).unwrap();

    let message = r#"{
      "parameters": [
        {
          "id": "source_paths",
          "type": "array_of_strings",
          "value": [
            "/tmp/source_1.json",
            "/tmp/source_2.json"
          ]
        },
        {
          "id": "mode",
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
          "value": "/tmp/destination.json"
        }
      ],
      "job_id": 124
    }"#;

    let job = Job::new(message).unwrap();
    let job_result = JobResult::new(job.job_id);
    let result = process(None, &job, job_result);

    let job_result = JobResult::new(124)
      .with_status(JobStatus::Error)
      .with_message(r#"IO Error: No such file: "/tmp/source_2.json""#);

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
            "/tmp/source.json"
          ]
        },
        {
          "id": "mode",
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
