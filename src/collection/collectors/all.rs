use crate::collection::buffers::WorkingBuffers;
use crate::collection::collectors::{cgroup_v1, Collector, StatWriter};
use crate::collection::perf_table::TableMetadata;
use crate::shared::CollectionMethod;
use anyhow::Error;

pub enum CollectorImpl {
    CgroupV1(cgroup_v1::Collector),
}

impl Collector for CollectorImpl {
    fn metadata(&mut self) -> Option<serde_yaml::Value> {
        match self {
            Self::CgroupV1(v1) => v1.metadata(),
        }
    }

    fn table_metadata(&mut self) -> TableMetadata {
        match self {
            Self::CgroupV1(v1) => v1.table_metadata(),
        }
    }

    fn get_type(&self) -> &'static str {
        match self {
            Self::CgroupV1(v1) => v1.get_type(),
        }
    }

    fn init(&mut self) -> Result<(), Error> {
        match self {
            Self::CgroupV1(v1) => v1.init(),
        }
    }

    fn write_header(&mut self, writer: &mut StatWriter) -> Result<(), csv::Error> {
        match self {
            Self::CgroupV1(v1) => v1.write_header(writer),
        }
    }

    fn collect(
        &mut self,
        writer: &mut StatWriter,
        working_buffers: &mut WorkingBuffers,
    ) -> Result<(), csv::Error> {
        match self {
            Self::CgroupV1(v1) => v1.collect(writer, working_buffers),
        }
    }
}

impl From<CollectionMethod> for CollectorImpl {
    fn from(method: CollectionMethod) -> Self {
        match method {
            CollectionMethod::LinuxCgroupV1(path) => {
                Self::CgroupV1(cgroup_v1::Collector::new(path))
            },
            CollectionMethod::LinuxCgroupV2(path) => todo!(),
        }
    }
}
