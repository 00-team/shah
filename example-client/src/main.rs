struct Taker;

impl shah::Taker for Taker {
    fn take(&self, order: &[u8]) -> Result<&[u8], shah::ErrorCode> {
        println!("taking the order: {order:?}");
        Err(example_db::models::ExampleError::UserNotFound)?
    }
}

fn main() {
    let taker = Taker;

    let user_gene = shah::Gene::default();

    let result = example_db::user::client::user_get(taker, user_gene);

    println!("result: {result:?}");
}
