use mixin_dekstop_core::sdk;
use sdk::Credential;
use sdk::KeyStore;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let a = sdk::Client::new(Credential::KeyStore(
        KeyStore {
            app_id: "a82e1a6a-75c9-4013-80bc-10183eef49a2".to_string(),
            session_id: "74ed2dbd-a2fa-4764-8718-f1489e6c4b07".to_string(),
            server_public_key: "e735290f8f0877c518f4483ccbb33cdd268386a82de0c63e18e3c595a0cde402".to_string(),
            session_private_key: "".to_string(),
        }
    ));
    let result = a.get_me().await;
    println!("a: {:?}", result)
}
