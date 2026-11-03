use std::sync::Arc;
use crate::installer::DownloadProgress;
use crate::launch::LaunchProgress;
use crate::crash::CrashAnalysisResult;

pub trait ProgressReporter: Send + Sync {
    fn emit_download(&self, progress: DownloadProgress);
    fn emit_launch(&self, progress: LaunchProgress);
    fn emit_crash(&self, analysis: CrashAnalysisResult);
    fn emit_exit(&self, exit_code: i32);
}

pub type ReporterHandle = Arc<dyn ProgressReporter>;

#[derive(Debug, Default, Clone)]
pub struct NoopProgressReporter;

impl ProgressReporter for NoopProgressReporter {
    fn emit_download(&self, _progress: DownloadProgress) {}
    fn emit_launch(&self, _progress: LaunchProgress) {}
    fn emit_crash(&self, _analysis: CrashAnalysisResult) {}
    fn emit_exit(&self, _exit_code: i32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_progress_reporter_invocations() {
        let reporter = NoopProgressReporter;
        reporter.emit_download(DownloadProgress {
            version_id: "test".to_string(),
            vanilla_version: None,
            step: "step".to_string(),
            percentage: 50.0,
            speed_text: "1MB/s".to_string(),
            is_finished: false,
            error: None,
        });
        reporter.emit_launch(LaunchProgress {
            step: "launching".to_string(),
            percentage: 20.0,
            pid: None,
            is_finished: false,
            error: None,
        });
        reporter.emit_crash(CrashAnalysisResult {
            exit_code: Some(1),
            title: "崩溃测试".to_string(),
            reason: "内存溢出".to_string(),
            suggestion: "增加最大内存".to_string(),
            log_snippet: "java.lang.OutOfMemoryError".to_string(),
            report_file: None,
        });
        reporter.emit_exit(0);
    }
}
