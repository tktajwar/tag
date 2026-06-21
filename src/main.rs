use clap::{Args, Parser, Subcommand};

use tag::PlainTag;

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

    /// Only include items with the given flags
    #[arg(short, long="with", default_value = None)]
    with_flags: Option<String>,

    /// Exclude items with the given flags
    #[arg(short='o', long="without", default_value = None)]
    without_flags: Option<String>,

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

    if let Some(flags) = cli.global_opts.with_flags {
	for flag in flags.trim().split(' ') {
	    let flag = flag.trim();
	    if flag.len() < 1 { continue };
	    let flag = if flag.starts_with('#') {
		flag.to_string()
	    } else {
		format!("#{flag}")
	    };
	    iter = iter.with_flag(flag);
	}
    }

    if let Some(flags) = cli.global_opts.without_flags {
	for flag in flags.trim().split(' ') {
	    let flag = flag.trim();
	    if flag.len() < 1 { continue };
	    let flag = if flag.starts_with('#') {
		flag.to_string()
	    } else {
		format!("#{flag}")
	    };
	    iter = iter.without_flag(flag);
	}
    }

    for item in iter {
	println!("{}", item);
    }

    Ok(())
}
