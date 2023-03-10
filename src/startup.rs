use std::net::TcpListener;
use actix_web::{web, App, HttpServer};
use actix_web::dev::Server;
use tracing_actix_web::TracingLogger;
use crate::routes;
use crate::configuration::Settings;
use secrecy::{Secret, ExposeSecret};
use mobc::Pool;
use mobc_redis::RedisConnectionManager;
use redis::RedisResult;
use tera::Tera;

use crate::RedisPool;
use crate::content_length_limit::ContentLengthLimit;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let redis_pool = get_redis_pool(configuration.redis_uri).await?;

        let tera = Tera::new("templates/**/*").expect("Failed to load page templates");
        
        let address = format!(
            "{}:{}",
            configuration.application.host,
            configuration.application.port,
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            redis_pool,
            tera,
        ).await?;

        Ok(Self{ port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn get_redis_pool(
    redis_uri: Secret<String>,
) -> RedisResult<RedisPool> {
    let client = redis::Client::open(redis_uri.expose_secret().as_ref())?;
    let manager = RedisConnectionManager::new(client);
    Ok(Pool::builder().max_open(100).build(manager))
}

pub async fn run(
    listener: TcpListener,
    redis_pool: RedisPool,
    tera: Tera,
) -> Result<Server, anyhow::Error> {
    let redis_pool = web::Data::new(redis_pool);
    let tera = web::Data::new(tera);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(ContentLengthLimit::default())
            .route("/health_check", web::get().to(routes::health_check))
            .route("/", web::get().to(routes::save_file_page))
            .route("/save", web::post().to(routes::save_file))
            .service(routes::load_file_page)  // Page to download any file
            .service(routes::load_file)  // GET any file by ID
            .service(routes::check_resource_state)  // Check if a file is ready
            .app_data(redis_pool.clone())
            .app_data(tera.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
