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

    /// Return all tag items from and up to/until the given IDs
    Between {
	/// The tag ID to start from
        #[command()]
        from_tagid: String,

	/// The tag ID to stop at
	#[command()]
	to_tagid: String,

	/// Fetch upto second tag ID instead of upto
	#[arg(short='x', long, default_value_t = false)]
	exclusive: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let ptag = PlainTag::try_from(args.file.as_str())?;

    let iter = match &args.command {
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

	    return Ok(())
	},
	Commands::Between {from_tagid, to_tagid, exclusive} => {
	    let from = tag::TagID::try_from(from_tagid.as_str())?;
	    let to = tag::TagID::try_from(to_tagid.as_str())?;
	    let iter = ptag.start_from(from);
	    if *exclusive {
		iter.until(to)
	    } else {
		iter.upto(to)
	    }
	},
    };

    for item in iter {
	println!("{}", item);
    }

    Ok(())
}
