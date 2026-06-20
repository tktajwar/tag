use crate::{
    TagItem,
    TagID,
};

use regex::Regex;
use std::sync::LazyLock;
use std::fs::File;
use memmap2::Mmap;
use std::error::Error;
use std::cmp::max;

static RE_TAG_ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(
	r"^\s*@([0-9]+\.?[0-9]*).*$"
    ).unwrap());

pub struct PlainTag {
    tagfile: Mmap,
    list_start: usize,
}

impl PlainTag {

    /// Returns an iterator over the tag items of the tag file.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let mut tag_items = ptag.items();
    /// ```

    pub fn items(&self) -> PTagIteratorConstrained<'_> {
	PTagIteratorConstrained {
	    iter: PTagIteratorType::NonConstrained(
		Box::new(PTagIterator {
		    iter: str::from_utf8(
			&self.tagfile[self.list_start..])
			.unwrap()
			.split('\n')
		})
	    ),
	    constraint: PTagIteratorConstraint::None,
	}
    }

    /// Returns the tag item with the given tag ID after performing a
    /// linear search, or `None` if it's not found.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let id = tag::TagID::try_from("@1.0").unwrap();
    /// let a = ptag.linear_search(id);
    /// ```

    pub fn linear_search(&self, id: TagID) -> Option<TagItem> {
	for item in self.items() {
	    if item.id == id {
		return Some(item);
	    }
	}
	None
    }

    /// Returns the tag item with the given tag ID after performing a
    /// binary search, or Error on ID parse failure or if the item
    /// isn't found.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let id = tag::TagID::try_from("@1.0").unwrap();
    /// let (a, pointer) = ptag.binary_search(id);
    /// ```

    pub fn binary_search(
	&self,
	id: TagID
    ) -> (Result<TagItem, Box<dyn Error>>, usize) {
	let mut s = self.list_start;
	let mut e = self.tagfile.len() as usize - 1;

	while s <= e {
	    let m = (s + e) / 2;
	    let (start, end) = self.line_range(m);
	    let m_item = match self.item_at(start, end) {
		Ok(item) => item,
		Err(e) => return (Err(e), s),
	    };
	    let m_id = m_item.id;

	    if id > m_id {
		s = max(end, m) + 1;
	    } else if id < m_id {
		e = start - 1;
	    } else {
		return (Ok(m_item), start);
	    }
	}

	(Err(Box::from(format!("Item {} Not Found.", id))), s)
    }

    fn line_range(
	&self,
	position: usize,
    ) -> (usize, usize) {
	let mut s = position;

	while s > self.list_start {
	    if self.tagfile[s] == b'@' && self.tagfile[s-1] == b'\n' {
		match self.tagfile[s+1] {
		    b'0'..=b'9' => {break;},
		    _ => (),
		}
	    }
	    s -= 1;
	}

	let mut e = s;

	while e < self.tagfile.len() {
	    if self.tagfile[e] == b'\n' {
		break;
	    }
	    e += 1;
	}

	(s, e)
    }

    fn item_at(
	&self,
	line_start: usize,
	line_end: usize,
    ) -> Result<TagItem, Box<dyn Error>> {
	let line = std::str::from_utf8(
	    &self.tagfile[line_start..line_end]
	)?;
	Ok(TagItem::try_from(line)?)
    }
}

impl TryFrom<&str> for PlainTag {
    type Error = Box<dyn Error>;

    /// Returns a PlainTag from given file path.
    ///
    /// # Errors
    ///
    /// This function will return an error if `filepath` does not
    /// already exist or when the underlying system call fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile");
    /// assert!(ptag.is_ok());
    /// ```

    fn try_from(filepath: &str) -> Result<PlainTag, Self::Error> {
	let tagfile = File::open(filepath)?;
	let tagfile = unsafe { Mmap::map(&tagfile)? };

	let mut list_start = 0;

	while list_start < tagfile.len() - 1 {
	    if tagfile[list_start] == b'@' {
		match tagfile[list_start + 1] {
		    b'0'..=b'9' => break,
		    _ => (),
		}
	    }

	    list_start += 1;
	}

	Ok( PlainTag {
	    tagfile,
	    list_start,
	} )
    }
}

pub struct PTagIterator<'a> {
    iter: std::str::Split<'a, char>,
}

impl <'a>Iterator for PTagIterator<'a> {
    type Item = TagItem;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
	for s in &mut self.iter {
	    if RE_TAG_ITEM.is_match(s) {
		match TagItem::try_from(s)  {
		    Ok(item) => return Some(item),
		    _ => continue,
		}
	    } else {
		continue;
	    }
	}

	return None;
    }
}

enum PTagIteratorConstraint {
    None,
    Until(TagID),
    Upto(TagID),
    WithFlag(String),
    WithoutFlag(String),
    MatchAttribute(Option<String>, Option<String>),
}

impl PTagIteratorConstraint {
    fn match_item(&self, item: &TagItem) -> bool {
	match self {
	    PTagIteratorConstraint::None => true,
	    PTagIteratorConstraint::Until(id) => item.id < *id,
	    PTagIteratorConstraint::Upto(id) => item.id <= *id,
	    PTagIteratorConstraint::WithFlag(flag) => item.has_flag(flag),
	    PTagIteratorConstraint::WithoutFlag(flag) => !item.has_flag(flag),
	    PTagIteratorConstraint::MatchAttribute(key, value) => {
		item.match_attribute((key.as_deref(), value.as_deref()))
	    },
	}
    }
}

enum PTagIteratorType<'a> {
    NonConstrained(Box<PTagIterator<'a>>),
    Constrained(Box<PTagIteratorConstrained<'a>>),
}

pub struct PTagIteratorConstrained<'a> {
    iter: PTagIteratorType<'a>,
    constraint: PTagIteratorConstraint,
}

impl<'a> PTagIteratorConstrained<'a> {

    /// Returns an iterator over the given current iterator with the
    /// constraint of iterating until the given ID.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let mut tag_items = ptag.items().until(
    ///     tag::TagID::try_from("@1000.0")
    ///         .expect("Failed to build tag ID")
    /// );
    /// ```

    pub fn until(self, id: TagID) -> PTagIteratorConstrained<'a> {
	PTagIteratorConstrained {
	    iter: PTagIteratorType::Constrained(
		Box::new(self)
	    ),
	    constraint: PTagIteratorConstraint::Until(id),
	}
    }

    /// Returns an iterator over the given current iterator with the
    /// constraint of iterating upto the given ID.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let mut tag_items = ptag.items().upto(
    ///     tag::TagID::try_from("@1000.0")
    ///         .expect("Failed to build tag ID")
    /// );
    /// ```

    pub fn upto(self, id: TagID) -> PTagIteratorConstrained<'a> {
	PTagIteratorConstrained {
	    iter: PTagIteratorType::Constrained(
		Box::new(self)
	    ),
	    constraint: PTagIteratorConstraint::Upto(id),
	}
    }

    /// Returns an iterator over the given iterator with the
    /// constraint of only returning items that contain the given
    /// flag.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let mut tag_items = ptag.items()
    ///     .with_flag("#TODO".to_string());
    /// ```

    pub fn with_flag(self, flag: String) -> PTagIteratorConstrained<'a> {
	PTagIteratorConstrained {
	    iter: PTagIteratorType::Constrained(
		Box::new(self)
	    ),
	    constraint: PTagIteratorConstraint::WithFlag(flag),
	}
    }

    /// Returns an iterator over the given iterator with the
    /// constraint of only returning items that do not contain the
    /// given flag.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let mut tag_items = ptag.items()
    ///     .without_flag("#DONE".to_string());
    /// ```

    pub fn without_flag(self, flag: String) -> PTagIteratorConstrained<'a> {
	PTagIteratorConstrained {
	    iter: PTagIteratorType::Constrained(
		Box::new(self)
	    ),
	    constraint: PTagIteratorConstraint::WithoutFlag(flag),
	}
    }

    /// Returns an iterator over the given iterator with the
    /// constraint of only returning items that has an attribute that
    /// match the given key and value.
    ///
    /// # Examples
    ///
    /// ```
    /// let ptag = tag::PlainTag::try_from("Tagfile").unwrap();
    /// let mut tag_items = ptag.items()
    ///     .match_attribute(
    ///         Some("file-type".to_string()),
    ///         Some("png".to_string()),
    ///     );
    /// ```

    pub fn match_attribute(
	self,
	key: Option<String>,
	value: Option<String>,
    ) -> PTagIteratorConstrained<'a> {
	PTagIteratorConstrained {
	    iter: PTagIteratorType::Constrained(
		Box::new(self)
	    ),
	    constraint: PTagIteratorConstraint::MatchAttribute(key, value),
	}
    }
}

impl <'a>Iterator for PTagIteratorConstrained<'a> {
    type Item = TagItem;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
	while let Some(item) = match &mut self.iter {
	    PTagIteratorType::NonConstrained(iter) => iter.next(),
	    PTagIteratorType::Constrained(iter) => iter.next(),
	} {
	    if self.constraint.match_item(&item) {
		return Some(item)
	    } else {
		match self.constraint {
		    PTagIteratorConstraint::Until(_) |
		    PTagIteratorConstraint::Upto(_) => return None,
		    _ => ()
		};
	    }
	}

	return None;
    }
}
