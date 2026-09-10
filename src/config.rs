use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_owned());
        let port = env::var("APP_PORT").unwrap_or_else(|_| "8080".to_owned());
        let bind_addr = format!("{host}:{port}")
            .parse()
            .map_err(|_| "APP_HOST/APP_PORT do not form a valid socket address".to_owned())?;
        let database_url =
            env::var("DATABASE_URL").map_err(|_| "DATABASE_URL is required".to_owned())?;
        Ok(Self {
            bind_addr,
            database_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_listen_using_app_host_app_port_and_database_url() {
        unsafe {
            env::set_var("APP_HOST", "127.0.0.1");
            env::set_var("APP_PORT", "3000");
            env::set_var("DATABASE_URL", "postgres://postgres:postgres@localhost/app");
        }
        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.bind_addr.to_string(), "127.0.0.1:3000");
        assert!(config.database_url.contains("postgres://"));
    }
}
