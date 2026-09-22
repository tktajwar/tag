use clap::{Args, Parser, Subcommand};
use regex::Regex;
use std::sync::LazyLock;

use tag::PlainTag;


static RE_FILTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(
	r"\s*([+-])\s*((#)[0-9a-zA-Z_]+|(:)(.+?)?:\s*([^+-]*[^\s+-]))\s*"
    ).unwrap());

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CLI {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[command(subcommand)]
    select: Select,
}

#[derive(Subcommand)]
enum Select {
    /// Return all items
    All,

    /// Return all tag items from the given ID
    From {
	/// The tag ID to start from
        #[command()]
        from_tagid: String,
    },

    /// Return all tag items upto the given ID
    Upto {
	/// The tag ID to return up to
        #[command()]
        to_tagid: String,
    },

    /// Return all tag items until the given ID
    Until {
	/// The tag ID to return until
        #[command()]
        to_tagid: String,
    },

    /// Return all tag items from and up to/until the given IDs
    Between {
	/// The tag ID to start from
        #[command()]
        from_tagid: String,

	/// The tag ID to stop at
	#[command()]
	to_tagid: String,

	/// Return upto second tag ID instead of upto
	#[arg(short='x', long, default_value_t = false)]
	exclusive: bool,
    },

    /// Search for tag items with given IDs [not filterable]
    Search {
	/// The IDs of the items to search for
        #[command()]
        tagids: Vec<String>,

	/// Use linear search instead of binary
	#[arg(long, default_value_t = false)]
	linear: bool,
    },
}

#[derive(Args)]
struct GlobalOpts {
    /// Path to the tagfile
    #[arg(short, long, default_value = "Tagfile")]
    file: String,

    /// Filter items
    #[arg(short='F', long, default_value = "")]
    filter: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = CLI::parse();
    let ptag = PlainTag::try_from(cli.global_opts.file.as_str())?;

    let mut iter = match &cli.select {
	Select::All => {
	    ptag.items()
	},

	Select::Search {tagids, linear} => {
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

	Select::From {from_tagid} => {
	    let from = tag::TagID::try_from(from_tagid.as_str())?;
	    ptag.start_from(from)
	},

	Select::Upto {to_tagid} => {
	    let to = tag::TagID::try_from(to_tagid.as_str())?;
	    ptag.items().upto(to)
	},

	Select::Until {to_tagid} => {
	    let to = tag::TagID::try_from(to_tagid.as_str())?;
	    ptag.items().until(to)
	},

	Select::Between {from_tagid, to_tagid, exclusive} => {
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

    for filter in RE_FILTER.captures_iter(&cli.global_opts.filter) {
	match (
	    filter.get(1).map(|s| s.as_str()),
	    filter.get(2).map(|s| s.as_str()),
	    filter.get(3).map(|s| s.as_str()),
	    filter.get(4).map(|s| s.as_str()),
	    filter.get(5).map(|s| s.as_str()),
	    filter.get(6).map(|s| s.as_str()),
	) {
	    (Some("+"), Some(flag), Some("#"), None, _, _) => {
		iter = iter.with_flag(flag.to_string())
	    },
	    (Some("-"), Some(flag), Some("#"), None, _, _) => {
		iter = iter.without_flag(flag.to_string())
	    },
	    (Some("+"), _, None, Some(":"), attribute, value) => {
		iter = iter.match_attribute(
		    attribute.map(|s| s.to_string()),
		    value.map(|s| s.to_string()),
		)
	    },
	    _ => eprintln!("Error parsing filter"),
	};
    }

    for item in iter {
	println!("{}", item);
    }

    Ok(())
}
