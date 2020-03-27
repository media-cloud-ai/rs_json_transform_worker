use amqp_worker::job::*;
use amqp_worker::MessageError;
use amqp_worker::ParametersContainer;
use jq_rs;
use lapin_futures::Channel;
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

pub fn process(
  _channel: Option<&Channel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  info!("Start process.");

  info!("Match on filter_type.");
  match job
    .get_string_parameter("filter_type")
    .unwrap_or_else(|| "string".to_string())
    .as_str()
  {
    // "file"      => filter_with_file(&job, job_result),
    "string" => filter_with_string(&job, job_result),
    "file" => filter_with_file(&job, job_result),
    filter_type => {
      let result = job_result
        .with_status(JobStatus::Error)
        .with_message(&format!("filter_type {:?} not supported (['file', 'string']).", filter_type));

      Err(MessageError::ProcessingError(result))
    }
  }
}

fn filter_with_string(job: &Job, job_result: JobResult) -> Result<JobResult, MessageError> {
  info!("Start filter_with_string.");

  info!("Parse source_paths.");
  let source_paths = get_source_paths(&job)?;

  info!("Parse destination path.");
  let destination_path = get_destination_path(&job)?;

  info!("Parse filter.");
  let filter = job.get_string_parameter("filter");
  if filter.is_none() {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message("Filter must be defined.");
    return Err(MessageError::ProcessingError(result));
  }

  filter_source_paths(
    &job,
    job_result,
    &source_paths,
    filter.unwrap().as_str(),
    &destination_path,
  )
}

fn filter_with_file(job: &Job, job_result: JobResult) -> Result<JobResult, MessageError> {
  info!("Start filter_with_file.");

  info!("Parse source_paths.");
  let source_paths = get_source_paths(&job)?;

  info!("Parse destination path.");
  let destination_path = get_destination_path(&job)?;

  info!("Parse filter file name.");
  let filter_filename = job
    .get_string_parameter("filter")
    .ok_or_else(|| {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message("Destination path must be defined.");
      MessageError::ProcessingError(result)
    })?;

  if Path::new(&filter_filename).is_file() {
    info!("Open filter file.");
    let filter_file = fs::File::open(&filter_filename).map_err(|e| {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message(&e.to_string());
      MessageError::ProcessingError(result)
    })?;

    info!("Read filter file content.");
    let mut buf_reader = BufReader::new(filter_file);
    let mut filter = String::new();
    buf_reader.read_to_string(&mut filter).map_err(|e| {
      let result = JobResult::from(job)
        .with_status(JobStatus::Error)
        .with_message(&e.to_string());
      MessageError::ProcessingError(result)
    })?;

    filter_source_paths(&job, job_result, &source_paths, &filter, &destination_path)
  } else {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&format!("{:?} is not an existing file.", filter_filename));
    return Err(MessageError::ProcessingError(result));
  }
}

fn filter_source_paths(
  job: &Job,
  job_result: JobResult,
  source_paths: &[String],
  filter: &str,
  destination_path: &str,
) -> Result<JobResult, MessageError> {
  info!("Start filter_source_paths.");

  info!("Compile filter as a jq program");
  let mut program = jq_rs::compile(filter).map_err(|e| {
    let result = JobResult::from(job)
      .with_status(JobStatus::Error)
      .with_message(&e.to_string());
    MessageError::ProcessingError(result)
  })?;

  info!("Run jq program on each source_paths.");
  for source_path in source_paths {
    let input_path = Path::new(&source_path);
    let output_path = Path::new(&destination_path);

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

      debug!("Create output file.");
      let mut output_file = fs::File::create(&output_path).map_err(|e| {
        let result = JobResult::from(job)
          .with_status(JobStatus::Error)
          .with_message(&e.to_string());
        MessageError::ProcessingError(result)
      })?;

      debug!("Write jq program result to file.");
      output_file
        .write_all(output_content.as_bytes())
        .map_err(|e| {
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
