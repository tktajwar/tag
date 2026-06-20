use clap::{Parser, Subcommand};

use tag::PlainTag;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the tagfile
    #[arg(short, long, default_value = "Tagfile")]
    file: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for tag items
    Search {
	/// The IDs of the items to search for
        #[command()]
        tagids: Vec<String>,

	/// Use linear search instead of binary
	#[arg(long, default_value_t = false)]
	linear: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ptag = PlainTag::try_from(args.file.as_str())?;

    match &args.command {
	Commands::Search {tagids, linear} => {
	    match linear {
		false => for tagid in tagids {
		    let id = tag::TagID::try_from(tagid.as_str())?;
		    if let (Ok(item), _) = ptag.binary_search(id) {
			println!("{}", item);
		    }
		},
		true => for tagid in tagids {
		    let id = tag::TagID::try_from(tagid.as_str())?;
		    if let Some(item) = ptag.linear_search(id) {
			println!("{}", item);
		    }
		},
	    }
	},
    }

    Ok(())
}
