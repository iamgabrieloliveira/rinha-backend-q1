use chrono::{DateTime, NaiveDateTime, offset::Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ClientTransactionRequest {
    pub valor: i32,
    pub tipo: String,
    pub descricao: String,
}

#[derive(Deserialize)]
pub enum TransactionType {
    Credit,
    Debit,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionStatement {
    pub valor: i32,
    pub tipo: String,
    pub descricao: String,
    pub realizada_em: NaiveDateTime,
}

#[derive(Serialize, Deserialize)]
pub struct Balance {
    pub total: i32,
    pub data_extrato: DateTime<Utc>,
    pub limite: i32,
}

#[derive(Serialize, Deserialize)]
pub struct CustomerStatementResponse {
    pub saldo: Balance,
    pub ultimas_transacoes: Vec<TransactionStatement>,
}

#[derive(Serialize)]
pub struct NewTransactionResponse {
    pub limite: i32,
    pub saldo: i32,
}
