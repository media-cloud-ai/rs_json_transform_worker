
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
    "Transform json using jq".to_string()
  }

  fn get_description(&self) -> String {
    r#"Manipulate json files to transform into desired format
    ."#
      .to_string()
  }

  fn get_version(&self) -> Version {
    semver::Version::parse(built_info::PKG_VERSION).expect("unable to locate Package version")
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    vec![
        Parameter {
            identifier: "source_path".to_string(),
            label: "Source path".to_string(),
            kind: vec![ParameterType::String],
            required: true,
        },
        Parameter {
            identifier: "filter".to_string(),
            label: "Filter".to_string(),
            kind: vec![ParameterType::String],
            required: true,
        },
        Parameter {
            identifier: "filter_type".to_string(),
            label: "Filter type".to_string(),
            kind: vec![ParameterType::String],
            required: true,
        },
        Parameter {
            identifier: "destination_path".to_string(),
            label: "Destination path".to_string(),
            kind: vec![ParameterType::String],
            required: true,
        }    
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