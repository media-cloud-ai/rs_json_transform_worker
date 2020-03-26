use amqp_worker::job::*;
use amqp_worker::MessageError;
use amqp_worker::ParametersContainer;
use lapin_futures::Channel;

pub fn process(
  _channel: Option<&Channel>,
  job: &Job,
  job_result: JobResult,
) -> Result<JobResult, MessageError> {
  match job
    .get_string_parameter("action")
    .unwrap_or_else(|| "Undefined".to_string())
    .as_str()
  {
    _ => hello_world(&job, job_result)
  }
}

fn hello_world(_job: &Job, job_result: JobResult) -> Result<JobResult, MessageError> {

    debug!("Hello world!");

    Ok(job_result.with_status(JobStatus::Completed))
  }