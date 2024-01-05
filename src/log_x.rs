use flexi_logger::{Duplicate, Criterion, Naming, Cleanup, Logger, FileSpec, DeferredNow};
use log::Record;

pub fn init_log(log_dir: &str, log_file: &str) {
    Logger::try_with_env_or_str("info").unwrap()
        .log_to_file(FileSpec::default()
            .directory(log_dir)
            .basename(log_file)
            .suffix(""))
        .duplicate_to_stdout(Duplicate::All)
        .rotate(
            Criterion::Size(10_000_000), // 例如, 每 10MB 切割
            Naming::Timestamps,
            Cleanup::KeepLogFiles(3), // 保留最新的3个日志文件
        )
        .append()
        .format(my_format)
        .create_symlink(format!("{}/{}", log_dir, log_file))
        .start()
        .unwrap();
}

fn my_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "{} [{}] {}",
        // now.format("%Y-%m-%d %H:%M:%S%.6f"),
        now.format("%Y-%m-%d %H:%M:%S"),
        record.level(),
        // record.file().unwrap_or("<unnamed>"),
        // record.line().unwrap_or(0),
        &record.args()
    )
}