use actix_web::{HttpServer, App, web, HttpResponse, Responder};
use serde_json::{json};
use std::env;
use deadpool_postgres::{ManagerConfig, RecyclingMethod, Runtime};
use deadpool_postgres::Pool;
use postgres::NoTls;
use chrono::{DateTime, NaiveDateTime, offset::Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ClientTransactionRequest {
    pub valor: i32,
    pub tipo: String,
    pub descricao: Option<String>,
}

fn env_or(key: &str, default: &str) -> Option<String> {
    Some(
        env::var(key).unwrap_or(default.to_string())
    )
}

fn get_db_config() -> deadpool_postgres::Config {
    let mut config = deadpool_postgres::Config::new();

    config.user = env_or("DB_USER", "admin");
    config.password = env_or("DB_PASSWORD", "1234");
    config.dbname = env_or("DB_NAME", "rinha");
    config.host = env_or("DB_HOST", "localhost");
    config.port = Some(5432);

    config.manager =
        Some(ManagerConfig { recycling_method: RecyclingMethod::Fast });

    config
}

fn create_pool() -> Result<Pool, String> {
    Ok(get_db_config().create_pool(Some(Runtime::Tokio1), NoTls).map_err(|err| err.to_string())?)
}

async fn customer_transaction(
    pool: web::Data<Pool>,
    path: web::Path<i32>,
    body: web::Json<ClientTransactionRequest>,
) -> impl Responder {
    let customer_id = path.into_inner();

    if customer_id < 1 || customer_id > 5 {
        return HttpResponse::NotFound().finish();
    }

    if body.tipo != "d" && body.tipo != "c" {
        return HttpResponse::UnprocessableEntity().finish();
    }

    let description = match &body.descricao {
        Some(desc) => desc,
        None => return HttpResponse::UnprocessableEntity().finish(),
    };

    if description.len() > 10 || description.len() < 1 {
        return HttpResponse::UnprocessableEntity().finish();
    }

    let is_debit = body.tipo.as_str() == "d";

    let value = if is_debit { -body.valor } else { body.valor };

    let conn = pool.get().await.unwrap();

    let result = conn
        .query("\
        UPDATE clientes SET saldo = saldo + $1 \
        WHERE id = $2 AND (saldo + $1) > (- limite) \
        RETURNING limite, saldo", &[&value, &customer_id])
        .await
        .unwrap();

    if result.is_empty() {
        return HttpResponse::UnprocessableEntity().finish();
    }

    let updated_customer = result.first().unwrap();

   conn
        .query("INSERT INTO transacoes(valor, id_cliente, tipo, descricao) values($1, $2, $3, $4)", &[
            &body.valor,
            &customer_id,
            &body.tipo,
            &body.descricao,
        ])
        .await
        .unwrap();

    HttpResponse::Ok()
        .body(
        json!({
            "limite": updated_customer.get::<_, i32>("limite"),
            "saldo": updated_customer.get::<_, i32>("saldo"),
        }).to_string()
    )
}

async fn customer_statement(
    pool: web::Data<Pool>,
    path: web::Path<i32>,
) -> impl Responder {
    let customer_id = path.into_inner();

    if customer_id < 1 || customer_id > 5 {
        return HttpResponse::NotFound().finish();
    }

    let conn = pool.get().await.unwrap();

    let extrato = conn.query_one("SELECT get_extrato($1)", &[&customer_id]).await.unwrap();

    HttpResponse::Ok()
        .body(
            extrato.get::<_, serde_json::Value>(0).to_string()
        )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env_or("PORT", "9999").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(create_pool().unwrap()))
            .route("/clientes/{client_id}/extrato", web::get().to(customer_statement))
            .route("/clientes/{client_id}/transacoes", web::post().to(customer_transaction))
        })
        .bind(format!("0.0.0.0:{port}"))?
        .run()
        .await
}
