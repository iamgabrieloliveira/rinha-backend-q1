mod db;
mod structs;

use std::thread;
use actix_web::{HttpServer, App, web, HttpResponse, Responder, post, get, error, HttpRequest};
use actix_web::web::JsonConfig;
use chrono::Utc;
use serde_json::{json};
use deadpool_postgres::{Pool};
use crate::structs::{ClientTransactionRequest, CustomerStatementResponse, Balance, TransactionStatement, NewTransactionResponse};

#[derive(Clone)]
struct AppData {
    db: Pool,
}

impl AppData {
    async fn get_database_connection(&self) -> deadpool_postgres::Object {
         self.db
            .get()
            .await
            .unwrap()
    }
}

fn is_between(value: usize, n1: usize, n2: usize) -> bool { value >= n1 && value <= n2 }

#[post("/clientes/{client_id}/transacoes")]
async fn customer_transaction(
    app: web::Data<AppData>,
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

    if ! is_between(body.descricao.len(), 1, 10) {
        return HttpResponse::UnprocessableEntity().finish();
    }

    let is_debit = body.tipo.as_str() == "d";

    let value = if is_debit { -body.valor } else { body.valor };

    let connection = app.get_database_connection().await;

    let result = connection
        .query("\
        UPDATE clientes SET saldo = saldo + $1 \
        WHERE id = $2 AND (saldo + $1) > (- limite) RETURNING limite, saldo", &[&value, &customer_id])
        .await
        .unwrap();

    if result.is_empty() {
        return HttpResponse::UnprocessableEntity().finish();
    }

    let updated_customer = result.first().unwrap();

    connection
        .query("INSERT INTO transacoes(valor, id_cliente, tipo, descricao) values($1, $2, $3, $4)", &[
            &body.valor,
            &customer_id,
            &body.tipo,
            &body.descricao,
        ])
        .await
        .unwrap();

    HttpResponse::Ok().json(
        NewTransactionResponse {
            limite: updated_customer.get("limite"),
            saldo: updated_customer.get("saldo"),
        }
    )
}

#[get("/clientes/{client_id}/extrato")]
async fn customer_statement(
    app: web::Data<AppData>,
    path: web::Path<i32>,
) -> impl Responder {
    let customer_id = path.into_inner();

    if customer_id < 1 || customer_id > 5 {
        return HttpResponse::NotFound().finish();
    }

    let conn = app.get_database_connection().await;

    let customer = conn.query_one(
        "SELECT json_build_object('total', saldo, 'limite', limite, 'data_extrato', now()) \
        FROM clientes \
        where id = $1",
        &[&customer_id]
    )
        .await
        .unwrap();

    let statements = conn.query(
        "SELECT json_build_object('valor', valor, 'tipo', tipo, 'descricao', descricao, 'realizada_em', realizada_em) \
        FROM transacoes \
        WHERE id_cliente = $1 \
        ORDER BY realizada_em \
        DESC LIMIT 10",
        &[&customer_id],
    )
        .await
        .unwrap();

    let statements_json: Vec<_> = statements
        .into_iter()
        .map(|row| row.get::<_, serde_json::Value>(0))
        .collect();

    let response = json!({
        "saldo": customer.get::<_, serde_json::Value>(0),
        "ultimas_transacoes": statements_json,
    });

    HttpResponse::Ok()
        .content_type("application/json")
        .body(response.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(
                AppData {
                    db: db::create_pool().unwrap(),
                }
            ))
            .service(customer_transaction)
            .service(customer_statement)
        })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
