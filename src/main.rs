#[macro_use]
extern crate log;

use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::{
  job::{Job, JobResult},
  start_worker, MessageError, MessageEvent,
};
use lapin_futures::Channel;
use semver::Version;

mod message;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Debug)]
struct JsonTransformEvent {}

impl MessageEvent for JsonTransformEvent {
  fn get_name(&self) -> String {
    "Json transform".to_string()
  }

  fn get_short_description(&self) -> String {
    "Transform json file(s) into another json based on template".to_string()
  }

  fn get_description(&self) -> String {
    r#"This worker enables the transformation of a json into an another json.
    The template is based on jq syntax to provide a very generic transformation tool.
    Input can be a single file to generate one transformed file.
    In case of multiple files is passed, a merged can be performed."#
      .to_string()
  }

  fn get_version(&self) -> Version {
    semver::Version::parse(built_info::PKG_VERSION).expect("unable to locate Package version")
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    vec![
      Parameter {
        identifier: "template_url".to_string(),
        label: "Template url".to_string(),
        kind: vec![ParameterType::String],
        required: true,
      },
      Parameter {
        identifier: "mode".to_string(),
        label: "Mode".to_string(),
        kind: vec![ParameterType::String],
        required: false,
      },
    ]
  }

  fn process(
    &self,
    channel: Option<&Channel>,
    job: &Job,
    job_result: JobResult,
  ) -> Result<JobResult, MessageError> {
    message::process(channel, job, job_result)
  }
}

static JSON_TRANSFORM_EVENT: JsonTransformEvent = JsonTransformEvent {};

fn main() {
  start_worker(&JSON_TRANSFORM_EVENT);
}
