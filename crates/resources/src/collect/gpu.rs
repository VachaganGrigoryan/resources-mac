//! Apple GPU usage from IOKit's `IOAccelerator` → `PerformanceStatistics`.
//! These keys are undocumented (the same ones Activity Monitor uses): anything missing yields `None`.

use std::collections::HashMap;
use std::ffi::CString;

use core_foundation::base::{CFType, TCFType, kCFAllocatorDefault};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use io_kit_sys::types::io_iterator_t;
use io_kit_sys::{
    IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperty, IOServiceGetMatchingServices,
    IOServiceMatching, kIOMasterPortDefault,
};

use crate::sample::History;

#[derive(Clone, Debug, Default)]
pub struct GpuStat {
    pub name: String,
    pub util_hist: History,
    pub mem_used: Option<u64>,
}

/// `(utilization %, memory in use)` from a `PerformanceStatistics` dictionary.
pub fn from_perf_stats(m: &HashMap<String, i64>) -> Option<(f64, Option<u64>)> {
    let util = *m.get("Device Utilization %")?;
    let mem = m
        .get("In use system memory")
        .and_then(|&v| u64::try_from(v).ok());
    Some((util.clamp(0, 100) as f64, mem))
}

/// Reads one registry property of `entry`, or `None` if absent.
fn property(entry: u32, key: &str) -> Option<CFType> {
    let key = CFString::new(key);
    // SAFETY: valid entry and key; the returned object follows the Create rule, so we own it.
    let raw = unsafe {
        IORegistryEntryCreateCFProperty(entry, key.as_concrete_TypeRef(), kCFAllocatorDefault, 0)
    };
    (!raw.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(raw) })
}

fn numeric_entries(dict: &CFDictionary) -> HashMap<String, i64> {
    let (keys, values) = dict.get_keys_and_values();
    keys.into_iter()
        .zip(values)
        .filter_map(|(k, v)| {
            // SAFETY: the dictionary holds CF objects; wrap_under_get_rule retains them.
            let (k, v) = unsafe {
                (
                    CFType::wrap_under_get_rule(k),
                    CFType::wrap_under_get_rule(v),
                )
            };
            Some((
                k.downcast::<CFString>()?.to_string(),
                v.downcast::<CFNumber>()?.to_i64()?,
            ))
        })
        .collect()
}

/// `(name, utilization %, memory in use)` of the first GPU that reports statistics.
pub fn read_gpu() -> Option<(String, f64, Option<u64>)> {
    let class = CString::new("IOAccelerator").ok()?;
    let mut iter: io_iterator_t = 0;
    // SAFETY: IOServiceMatching returns a dictionary that IOServiceGetMatchingServices consumes.
    let kr = unsafe {
        IOServiceGetMatchingServices(
            kIOMasterPortDefault,
            IOServiceMatching(class.as_ptr()),
            &mut iter,
        )
    };
    if kr != 0 {
        return None;
    }
    let mut found = None;
    loop {
        // SAFETY: iter is a valid iterator until released below.
        let entry = unsafe { IOIteratorNext(iter) };
        if entry == 0 {
            break;
        }
        if found.is_none() {
            let stats = property(entry, "PerformanceStatistics")
                .and_then(|t| t.downcast::<CFDictionary>())
                .map(|d| numeric_entries(&d));
            if let Some((util, mem)) = stats.as_ref().and_then(from_perf_stats) {
                let name = property(entry, "model")
                    .and_then(|t| t.downcast::<CFString>())
                    .map_or_else(|| "GPU".to_owned(), |s| s.to_string());
                found = Some((name, util, mem));
            }
        }
        // SAFETY: every object from IOIteratorNext must be released exactly once.
        unsafe { IOObjectRelease(entry) };
    }
    // SAFETY: release the iterator we own.
    unsafe { IOObjectRelease(iter) };
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn from_perf_stats_reads_util() {
        let m = HashMap::from([
            ("Device Utilization %".to_owned(), 37),
            ("In use system memory".to_owned(), 1_048_576),
        ]);
        assert_eq!(from_perf_stats(&m), Some((37.0, Some(1_048_576))));
    }

    #[test]
    fn from_perf_stats_missing_is_none() {
        let m = HashMap::from([("Renderer Utilization %".to_owned(), 5)]);
        assert_eq!(from_perf_stats(&m), None);
    }

    #[test]
    fn read_gpu_does_not_panic() {
        let _ = read_gpu();
    }
}
