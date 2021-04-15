use clap::Clap;

mod utils;
mod near;

#[derive(Clap, Debug, Clone)]
#[clap(version = "0.2.0", author = "Near Inc. <hello@nearprotocol.com>")]
pub(crate) struct Opts {
    #[clap(short, long)]
    pub lockup_account_id: String,
    #[clap(short)]
    pub block_height: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let account = opts.lockup_account_id;
    let block = near::rpc::get_block(opts.block_height).await?;

    let block_height = block.height;

    println!("owner_account_id,account_id,initial_lockup_amount,current_locked_amount");

    loop {
        match near::rpc::get_account_state(account.to_string(), block_height).await {
            Ok(option_state) => {
                if let Some(state) = option_state {
                    println!("{:#?}", state);
                    let locked_amount = state.get_locked_amount(block.timestamp).0;
                    println!("{},{},{},{}",
                        state.owner_account_id,
                        account,
                        utils::human(state.lockup_information.lockup_amount),
                        utils::human(locked_amount)
                    );
                }
                break;
            },
            Err(_err) => {},
        }
    }

    Ok(())
}
