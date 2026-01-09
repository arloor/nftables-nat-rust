use env_logger::Env;

pub fn init(env_cargo_crate_name: &str) {
    use chrono::Local;
    use env_logger;
    use std::io::Write;

    let default_filter = if cfg!(debug_assertions) {
        format!("info,{env_cargo_crate_name}=debug")
    } else {
        format!("error,{env_cargo_crate_name}=info")
    };
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or(&default_filter))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .try_init();
}
