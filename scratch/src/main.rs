use std::str::FromStr;

use hiero_sdk::{
    AccountId,
    Client,
    PrivateKey,
    TopicCreateTransaction,
};

#[tokio::main]
async fn main() {
    println!("starting...");

    let operator_id = AccountId::from_str("0.0.2").unwrap();
    println!("operator id parsed");

    let operator_key = PrivateKey::from_str_der(
        "302e020100300506032b65700422042091132178e72057a1d7528025956fe39b0b847f200ab59b2fdd367017f3087137"
    ).unwrap();
    println!("private key parsed");

    let mut network = std::collections::HashMap::new();

    network.insert("127.0.0.1:50211".to_string(), AccountId::from(3));

    println!("network configured");

    let client = Client::for_network(network).unwrap();
    println!("client created");

    client.set_operator(operator_id, operator_key);
    println!("operator set");

    let mut tx = TopicCreateTransaction::new();
    println!("transaction created");

    println!("executing transaction...");

    let resp = match tx.execute(&client).await {
        Ok(v) => {
            println!("transaction submitted");
            v
        }
        Err(e) => {
            eprintln!("execute failed: {:?}", e);
            return;
        }
    };

    println!("fetching receipt...");

    let receipt = match resp.get_receipt(&client).await {
        Ok(v) => {
            println!("receipt received");
            v
        }
        Err(e) => {
            eprintln!("receipt failed: {:?}", e);
            return;
        }
    };

    println!("SUCCESS");
    println!("topic id = {:?}", receipt.topic_id);
}
