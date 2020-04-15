use amqp_worker::job::*;
use amqp_worker::MessageError;
use amqp_worker::ParametersContainer;
use jq_rs;
use lapin_futures::Channel;
use std::fs;
use std::io::BufReader;
use std::path::Path;

pub fn process(
  _channel: Option<&Channel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  info!("Start process.");

  info!("Match on process mode.");
  let template_content = match job
    .get_string_parameter("mode")
    .unwrap_or_else(|| "string".to_string())
    .as_str()
  {
    "string" => get_template_from_string(&job)?,
    "file" => get_template_from_url(&job)?,
    mode => {
      let result = job_result
        .with_status(JobStatus::Error)
        .with_message(&format!(
          "Mode '{}' not supported (['file', 'string']).",
          mode
        ));

      return Err(MessageError::ProcessingError(result));
    }
  };

  info!("Parse source_paths.");
  let source_paths = get_source_paths(&job)?;

  info!("Parse destination path.");
  let destination_path = get_destination_path(&job)?;

  info!("Match on slurp flag.");
  match job.get_boolean_parameter("slurp").unwrap_or_else(|| false) {
    true => process_source_paths_with_slurp(
      &job,
      job_result,
      &source_paths,
      &template_content,
      &destination_path,
    ),
    false => process_source_paths_without_slurp(
      &job,
      job_result,
      &source_paths,
      &template_content,
      &destination_path,
    ),
  }
}

fn get_template_from_string(job: &Job) -> Result<String, MessageError> {
  info!("Start process_with_string.");

  info!("Parse template_url.");
  job.get_string_parameter("template_url").ok_or_else(|| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message("Filter must be defined.");
    MessageError::ProcessingError(result)
  })
}

fn get_template_from_url(job: &Job) -> Result<String, MessageError> {
  info!("Start process_with_source_url.");

  info!("Parse template_url.");
  let template_filename = job.get_string_parameter("template_url").ok_or_else(|| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message("Destination path must be defined.");
    MessageError::ProcessingError(result)
  })?;

  if !Path::new(&template_filename).is_file() {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&format!("{:?} is not an existing file.", template_filename));
    return Err(MessageError::ProcessingError(result));
  }

  info!("Read template_url.");
  fs::read_to_string(&template_filename).map_err(|e| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })
}

fn process_source_paths_without_slurp(
  job: &Job,
  job_result: JobResult,
  source_paths: &[String],
  template_content: &str,
  destination_path: &str,
) -> Result<JobResult, MessageError> {
  info!("Start process_source_paths_without_slurp.");

  info!("Compile template_content as a jq program");
  let mut program = jq_rs::compile(template_content).map_err(|e| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  info!("Run jq program on each source_paths.");
  for source_path in source_paths {
    let input_path = Path::new(&source_path);

    info!("{}", format!("Run jq program on '{}'.", source_path));
    if input_path.is_file() {
      debug!("Parse file content.");
      let file_content = fs::read_to_string(input_path).map_err(|e| {
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })?;

      debug!("Run jq program.");
      let output_content = &program.run(&file_content.to_string()).map_err(|e| {
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })?;

      debug!("Write jq program result to destination_path.");
      fs::write(&destination_path, output_content).map_err(|e| {
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })?;
    } else if input_path.is_dir() {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message("Source path must be a file.");
      return Err(MessageError::ProcessingError(result));
    } else {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message(&format!("No such a file or directory: '{:?}'", input_path));
      return Err(MessageError::ProcessingError(result));
    }
  }

  Ok(job_result.with_status(JobStatus::Completed))
}

fn process_source_paths_with_slurp(
  job: &Job,
  job_result: JobResult,
  source_paths: &[String],
  template_content: &str,
  destination_path: &str,
) -> Result<JobResult, MessageError> {
  info!("Start process_source_paths_with_slurp.");

  info!("Compile template_content as a jq program");
  let mut program = jq_rs::compile(template_content).map_err(|e| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  let mut input_contents = Vec::new();

  info!("Read content of all files.");
  for source_path in source_paths {
    let input_path = Path::new(&source_path);
    if input_path.is_file() {

      let file = fs::File::open(input_path).map_err(|e| {
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })?;
      let reader = BufReader::new(file);

      debug!("{}", format!("Parse file content '{}'.", source_path));
      let file_content =serde_json::from_reader(reader).map_err(|e| {
        error!("{}", &e.to_string());
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })?;
      
      debug!("Append to input_contents");
      input_contents.push(file_content);

    } else if input_path.is_dir() {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message("Source path must be a file.");
      return Err(MessageError::ProcessingError(result));
    } else {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message(&format!("No such a file or directory: '{:?}'", input_path));
      return Err(MessageError::ProcessingError(result));
    }
  }

  info!("Concatenate all the inputs.");
  let content = serde_json::to_string(&input_contents).map_err(|e| {
    let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message(&e.to_string());
      MessageError::ProcessingError(result)
  })?;

  debug!("{}", content);

  info!("Run jq program on slurped files.");
  let output_content = &program.run(&content).map_err(|e| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  debug!("Write jq program result to destination_path.");
  fs::write(&destination_path, output_content).map_err(|e| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  Ok(job_result.with_status(JobStatus::Completed))
}

fn get_source_paths(job: &Job) -> Result<Vec<String>, MessageError> {
  let source_paths = if let Some(source_path) = job.get_string_parameter("source_path") {
    vec![source_path]
  } else {
    job
      .get_array_of_strings_parameter("source_paths")
      .ok_or_else(|| {
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message("Either source_path either source_paths must be defined.");
        MessageError::ProcessingError(result)
      })?
  };

  Ok(source_paths)
}

fn get_destination_path(job: &Job) -> Result<String, MessageError> {
  let destination_path = job
    .get_string_parameter("destination_path")
    .ok_or_else(|| {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message("Destination path must be defined.");
      MessageError::ProcessingError(result)
    })?;

  Ok(destination_path)
}

// fn get_source_paths_contents(_job: &Job, _source_paths: &[String]) -> Result<String, MessageError> {
//   let result = String::from("");

//   Ok(result)
// }

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json;
  use std::fs::File;
  use std::io::Write;

  fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
  }

  #[test]
  fn process_with_string_test_ok() {
    init();
    let john = serde_json::json!({
      "name": "John Doe",
      "age": 43,
      "phones": [
          "+44 1234567",
          "+44 2345678"
      ]
    });

    let source_path = Path::new("/tmp/source.json");
    let source_file = File::create(source_path).unwrap();
    serde_json::to_writer(source_file, &john).unwrap();
    assert!(source_path.exists());

    let destination_path = Path::new("/tmp/destination.json");

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
          "value": "string"
        },
        {
          "id": "template_url",
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

    assert!(result.is_ok());
    assert!(destination_path.exists());
    assert_eq!(
      fs::read_to_string(&destination_path).unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn process_with_template_url_test_ok() {
    init();
    let john = serde_json::json!({
      "name": "John Doe",
      "age": 43,
      "phones": [
          "+44 1234567",
          "+44 2345678"
      ]
    });

    let source_path = Path::new("/tmp/source.json");
    let source_file = File::create(source_path).unwrap();
    serde_json::to_writer(source_file, &john).unwrap();
    assert!(source_path.exists());

    let template_path = Path::new("/tmp/template.jq");
    let mut template_file = File::create(template_path).unwrap();
    template_file.write_all(".name".as_bytes()).unwrap();
    assert!(template_path.exists());

    let destination_path = Path::new("/tmp/destination.json");

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
          "id": "template_url",
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
    assert!(destination_path.exists());
    assert_eq!(
      fs::read_to_string(&destination_path).unwrap(),
      "\"John Doe\"\n"
    );
  }

  #[test]
  fn slurp_test_ok() {
    init();
    let hello = serde_json::json!({
      "word": "Hello, "
    });

    let source_path1 = Path::new("/tmp/source1.json");
    let source_file = File::create(source_path1).unwrap();
    serde_json::to_writer(source_file, &hello).unwrap();
    assert!(source_path1.exists());

    let world = serde_json::json!({
      "name": "world!"
    });

    let source_path2 = Path::new("/tmp/source2.json");
    let source_file = File::create(source_path2).unwrap();
    serde_json::to_writer(source_file, &world).unwrap();
    assert!(source_path2.exists());

    let destination_path = Path::new("/tmp/destination.json");

    let message = r#"{
      "parameters": [
        {
          "id": "source_paths",
          "type": "array_of_strings",
          "value": [
            "/tmp/source1.json",
            "/tmp/source2.json"
          ]
        },
        {
          "id": "mode",
          "type": "string",
          "value": "string"
        },
        {
          "id": "slurp",
          "type": "boolean",
          "value": true
        },
        {
          "id": "template_url",
          "type": "string",
          "value": ".[0].word + .[1].word"
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
    assert!(destination_path.exists());
    assert_eq!(
      fs::read_to_string(&destination_path).unwrap(),
      "\"Hello, world!\"\n"
    );
  }

  #[test]
  fn process_test_error() {
    init();
    let john = serde_json::json!({
      "name": "John Doe",
      "age": 43,
      "phones": [
          "+44 1234567",
          "+44 2345678"
      ]
    });

    let source_path_1 = Path::new("/tmp/source_1.json");
    let source_file_1 = File::create(source_path_1).unwrap();
    serde_json::to_writer(source_file_1, &john).unwrap();
    assert!(source_path_1.exists());

    let source_path_2 = Path::new("/tmp/source_2.json");
    assert!(!source_path_2.exists());

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
          "id": "template_url",
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
      .with_message(&format!(
        "No such a file or directory: '{:?}'",
        source_path_2
      ));

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
      .with_message("Mode 'url' not supported (['file', 'string']).");

    assert_eq!(result, Err(MessageError::ProcessingError(job_result)));
  }
}
