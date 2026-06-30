use std::fmt;
use std::sync::LazyLock;
use regex::Regex;
use rust_decimal::prelude::*;

static RE_TAG_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(
	r"^\s*@([0-9]+\.?[0-9]*)\s*$"
    ).unwrap());

static RE_FLAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(
	r"#[0-9a-zA-Z_]+"
    ).unwrap());

static RE_FLAGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(
	r"^\s*(#[0-9a-zA-Z_]+(?:\s*#[0-9a-zA-Z_]+)*)\s*$"
    ).unwrap());

static RE_ATTRIBUTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(
	r"^\s*:(.+?)?:\s*(.+?)?\s*$"
    ).unwrap());


#[derive(PartialEq, PartialOrd, Eq, Clone, Debug, Copy)]
pub struct TagID {
    id: Decimal,
}

impl TryFrom<&str> for TagID {
    type Error = rust_decimal::Error;

    /// Returns a TagID from string slice, or `rust_decimal::Error` on
    /// parse failure.
    ///
    /// # Examples
    ///
    /// ```
    /// let id1 = tag::TagID::try_from("@1.0");
    ///
    /// assert!(id1.is_ok());
    /// ```
    ///
    /// Leading and trailing zeros will be ignored.
    ///
    /// ```
    /// assert_eq!(
    ///     tag::TagID::try_from("@00000000000000000001"),
    ///     tag::TagID::try_from("@1.000000000000000000"),
    /// );
    /// ```

    fn try_from(tag_number: &str) -> Result<TagID, Self::Error> {
	let tag_number = match RE_TAG_NUMBER.captures(tag_number) {
	    Some(captures) => captures.get(1).unwrap().as_str(),
	    None => "",
	};

	let id = Decimal::from_str_exact(
	    &tag_number
	)?.normalize();

	Ok (
	    TagID {
		id,
	    }
	)
    }
}

impl fmt::Display for TagID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	if self.id.scale() > 0 {
	    write!(f, "@{}", self.id.to_string())
	} else {
	    write!(f, "@{}.0", self.id.to_string())
	}
    }
}

impl TagID {
    /// Returns a TagID that comes after a TagID
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagID::try_from("@1.0").unwrap();
    /// let b = tag::TagID::generate_next(&a);
    ///
    /// assert_eq!(tag::TagID::try_from("@2.0").unwrap(), b);
    /// assert!(a < b);
    /// ```
    ///
    /// ```
    /// let a = tag::TagID::try_from("@1.1").unwrap();
    /// let b = tag::TagID::generate_next(&a);
    ///
    /// assert_eq!(tag::TagID::try_from("@1.2").unwrap(), b);
    /// assert!(a < b);
    /// ```
    ///
    /// ```
    /// let a = tag::TagID::try_from("@1.9").unwrap();
    /// let b = tag::TagID::generate_next(&a);
    ///
    /// assert_eq!(tag::TagID::try_from("@2").unwrap(), b);
    /// assert!(a < b);
    /// ```

    pub fn generate_next(tag_id: &TagID) -> TagID {
	let id = tag_id.id;
	let next_id = id + Decimal::new(1, id.scale());

	TagID {
	    id: next_id,
	}
    }

    /// Returns a TagID between two TagIDs.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagID::try_from("@1.0").unwrap();
    /// let d = tag::TagID::try_from("@2.0").unwrap();
    /// let b = tag::TagID::generate_between(&a, &d);
    /// let c = tag::TagID::generate_between(&b, &d);
    ///
    /// assert!(a < b && b < c && c < d);
    /// ```

    pub fn generate_between(
	id1: &TagID, id2: &TagID
    ) -> TagID {
	let smaller_id;
	let larger_id;

	if id1 < id2 {
	    smaller_id = id1;
	    larger_id = id2;
	} else if id2 < id1 {
	    smaller_id = id2;
	    larger_id = id1;
	} else {
	    return id1.clone();
	}

	let middle_id = TagID::generate_next(smaller_id);
	if middle_id < *larger_id {
	    return middle_id;
	}

	TagID{
	    id: (
		(smaller_id.id / Decimal::TWO) + (larger_id.id / Decimal::TWO)
	    ),
	}
    }
}

#[derive(PartialEq)]
#[derive(Debug)]
pub enum TagField<'a> {
    Invalid(&'a str),
    ID(&'a str),
    Title(&'a str),
    Flags(&'a str),
    Attribute(&'a str),
}

impl <'a>TagField<'a> {
    /// Return a field with the appropriate type from the given str.

    fn from(field_str: &'a str) -> TagField<'a> {
	if field_str.len() == 0 {
	    return TagField::Invalid(field_str)
	}

	let first_byte: u8 = {
	    let mut first: usize = 0;
	    while first+1 < field_str.len() {
		if field_str.as_bytes()[first] != b' ' {break};
		first += 1;
	    }
	    field_str.as_bytes()[first]
	};

	match first_byte {
	    b'@' => {
		if RE_TAG_NUMBER.is_match(field_str) {
		    TagField::ID(field_str)
		} else {
		    TagField::Title(field_str)
		}
	    },
	    b'#' => {
		if RE_FLAGS.is_match(field_str) {
		    TagField::Flags(field_str)
		} else {
		    TagField::Invalid(field_str)
		}
	    },
	    b':' => {
		if RE_ATTRIBUTE.is_match(field_str) {
		    TagField::Attribute(field_str)
		} else {
		    TagField::Invalid(field_str)
		}
	    },
	    b' ' => TagField::Invalid(field_str),
	    _   => TagField::Title(field_str),
	}
    }

    /// Returns a vector of flags of the field, or `None` if the field
    /// isn't flags type.
    ///
    /// # Examples
    ///
    /// ```
    /// let f1 = tag::TagField::Flags("#hello #world");
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#hello".to_string(),
    ///         "#world".to_string(),
    ///     ]),
    ///     f1.flags(),
    /// );
    ///
    /// let f2 = tag::TagField::Attribute(":atr: value");
    ///
    /// assert_eq!(None, f2.flags());
    /// ```

    pub fn flags(&self) -> Option<Vec<String>> {
	match self {
	    TagField::Flags(field_str) => Some(RE_FLAG.find_iter(field_str)
					       .map(|m| m.as_str().to_string())
					       .collect()),
	    _ => None,
	}
    }

    /// Returns String of flags after concatenating with the field's
    /// flags, or `None` if `flags` argument is invalid or if the
    /// field isn't of flag type.
    ///
    /// # Examples
    ///
    /// ```
    /// let f1 = tag::TagField::Flags("#hello #world");
    /// assert_eq!(
    ///     Some("#hello #world #rust #program".to_string()),
    ///     f1.concat_flags("#rust #program"),
    /// );
    /// assert_eq!(
    ///     None,
    ///     f1.concat_flags(":attr: value"),
    /// );
    /// ```
    ///
    /// ```
    /// let f2 = tag::TagField::Title("Not a TagField::Flags");
    /// assert_eq!(None, f2.concat_flags("#rust #program"));
    /// ```

    pub fn concat_flags(&self, flags: &str) -> Option<String> {
	let Some(flags) = RE_FLAGS.find(flags) else {
	    return None;
	};
	let con_flags = flags.as_str();

	match self {
	    TagField::Flags(field_str) => Some(field_str.to_string() + " " + con_flags),
	    _ => None,
	}
    }

    /// Returns String of flags after removing the flags from the
    /// field, or `None` if if the field isn't of flag type.
    ///
    /// # Examples
    ///
    /// ```
    /// let f1 = tag::TagField::Flags("#hello #world");
    /// assert_eq!(
    ///     Some("#hello".to_string()),
    ///     f1.sincat_flags("#world"),
    /// );
    /// assert_eq!(
    ///     Some("#hello #world".to_string()),
    ///     f1.sincat_flags(":attr: value"),
    /// );
    /// ```
    ///
    /// ```
    /// let f2 = tag::TagField::Title("Not a TagField::Flags");
    /// assert_eq!(None, f2.sincat_flags("#hello #world"));
    /// ```

    pub fn sincat_flags(&self, flags: &str) -> Option<String> {
	let sin_flags: Vec<String> = RE_FLAG.find_iter(&flags)
	    .map(|m| m.as_str().to_string())
	    .collect();

	match self {
	    TagField::Flags(_) => (),
	    _ => return None,
	};

	let mut flags_sinned = String::with_capacity(
	    flags.len()
	);
	flags_sinned.push(' ');
	let Some(old_flags) = self.flags() else {
	    return None;
	};
	for flag in old_flags {
	    if !sin_flags.contains(&flag) {
		flags_sinned.push_str(&(flag + " "));
	    }
	}

	Some(flags_sinned.trim().to_string())
    }

    /// Returns the attribute key (String), or `None` if it's not an
    /// attribute.
    ///
    /// # Examples
    ///
    /// ```
    /// let f1 = tag::TagField::Attribute(":src: code");
    /// assert_eq!(Some("src".to_string()), f1.attribute_key());
    /// ```
    ///
    /// ```
    /// let f2 = tag::TagField::Title("hello world");
    /// assert_eq!(None, f2.attribute_key());
    /// ```

    pub fn attribute_key(&self) -> Option<String> {
	match self {
	    TagField::Attribute(field_str) => {
		let Some(captures) = RE_ATTRIBUTE.captures(field_str) else {
		    return None
		};
		if let Some(key) = captures.get(1) {
		    Some(key.as_str().to_string())
		} else {
		    None
		}
	    },
	    _ => None,
	}
    }

    /// Returns the attribute value (String), or `None` if it's not an
    /// attribute or if the value is void.
    ///
    /// # Examples
    ///
    /// ```
    /// let f1 = tag::TagField::Attribute(":src: code");
    /// assert_eq!(Some("code".to_string()), f1.attribute_value());
    /// ```
    ///
    /// ```
    /// let f2 = tag::TagField::Title("hello world");
    /// assert_eq!(None, f2.attribute_value());
    /// ```
    ///
    /// ```
    /// let f3 = tag::TagField::Attribute(":existentialism:");
    /// assert_eq!(None, f3.attribute_value());
    /// ```

    pub fn attribute_value(&self) -> Option<String> {
	match self {
	    TagField::Attribute(field_str) => {
		let Some(captures) = RE_ATTRIBUTE.captures(field_str) else {
		    return None
		};
		if let Some(value) = captures.get(2) {
		    Some(value.as_str().to_string())
		} else {
		    None
		}
	    },
	    _ => None,
	}
    }

    /// Returns both attribute key and value, or `None` if it's not an
    /// attribute.
    ///
    /// # Examples
    ///
    /// ```
    /// let f1 = tag::TagField::Attribute(":src: code");
    /// assert_eq!(
    ///     Some((
    ///         Some("src".to_string()),
    ///         Some("code".to_string()),
    ///     )),
    ///     f1.attribute_key_value(),
    /// );
    /// ```
    ///
    /// ```
    /// let f2 = tag::TagField::Title("hello world");
    /// assert_eq!(None, f2.attribute_key_value());
    /// ```
    ///
    /// ```
    /// let f3 = tag::TagField::Attribute(":existentialism:");
    /// assert_eq!(
    ///     Some((
    ///         Some("existentialism".to_string()),
    ///         None,
    ///     )),
    ///     f3.attribute_key_value(),
    /// );
    /// ```

    pub fn attribute_key_value(
	&self
    ) -> Option<(Option<String>, Option<String>)> {
	match self {
	    TagField::Attribute(field_str) => {
		let Some(captures) = RE_ATTRIBUTE.captures(field_str) else {
		    return None
		};
		Some((
		    if let Some(key) = captures.get(1) {
			Some(key.as_str().to_string())
		    } else {
			None
		    },
		    if let Some(value) = captures.get(2) {
			Some(value.as_str().to_string())
		    } else {
			None
		    },
		))
	    },
	    _ => None,
	}
    }
}

pub struct FieldsIterator<'a> {
    iter: std::str::Split<'a, char>
}

impl <'a>Iterator for FieldsIterator<'a> {
    type Item = TagField<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
	if let Some(s) = self.iter.next() {
	    Some(TagField::from(s))
	} else {
	    None
	}
    }
}

#[derive(Clone)]
pub struct TagItem {
    pub id: TagID,
    tag_line: String,
}

impl TagItem {

    /// Returns an iterator over the pipe-separated fields of the
    /// `TagItem`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | My Title | #hello #world | :src: code | \
    ///      :invalid | #valid_flag | #inva!!lid"
    /// ).unwrap();
    /// let mut fields_iter = a.fields();
    ///
    /// assert_eq!(
    ///     Some(tag::TagField::ID("@1.0 ")),
    ///     fields_iter.next(),
    /// );
    /// assert_eq!(
    ///     Some(tag::TagField::Title(" My Title ")),
    ///     fields_iter.next(),
    /// );
    /// assert_eq!(
    ///     Some(tag::TagField::Flags(" #hello #world ")),
    ///     fields_iter.next(),
    /// );
    /// assert_eq!(
    ///     Some(tag::TagField::Attribute(" :src: code ")),
    ///     fields_iter.next(),
    /// );
    /// assert_eq!(
    ///     Some(tag::TagField::Invalid(" :invalid ")),
    ///     fields_iter.next(),
    /// );
    /// assert_eq!(
    ///     Some(tag::TagField::Flags(" #valid_flag ")),
    ///     fields_iter.next()
    /// );
    /// assert_eq!(
    ///     Some(tag::TagField::Invalid(" #inva!!lid")),
    ///     fields_iter.next(),
    /// );
    /// assert_eq!(None, fields_iter.next());
    /// ```

    pub fn fields(&self) -> FieldsIterator<'_> {
	FieldsIterator {
	    iter: self.tag_line.split('|'),
	}
    }

    /// Returns a vector of flags of the given TagItem.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | My Title | #hello #world | \
    ///      :invalid | #valid_flag | #inva!!lid"
    /// ).unwrap();
    /// assert_eq!(
    ///     Some(vec![
    ///         "#hello".to_string(),
    ///         "#world".to_string(),
    ///         "#valid_flag".to_string(),
    ///     ]),
    ///     a.flags(),
    /// );
    /// ```
    ///
    /// ```
    /// let b = tag::TagItem::try_from("@1.0 | Item with no flags").unwrap();
    /// assert_eq!(None, b.flags());
    /// ```

    pub fn flags(&self) -> Option<Vec<String>> {
	let mut flags: Vec<String> = Vec::new();

	for field in self.fields() {
	    match field.flags() {
		Some(new_flags) => flags.extend(new_flags.to_owned()),
		None => (),
	    }
	}

	if flags.len() != 0 {
	    Some(flags)
	} else {
	    None
	}
    }

    /// Returns `true` if the item has the given flags.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | My Title | #hello #world |\
    ///      :invalid | #valid_flag | #inva!!lid"
    /// ).unwrap();
    ///
    /// assert!(!(a.has_flag(&"#test".to_string())));
    ///
    /// let b = tag::TagItem::try_from("@1.0 | Item with no flags").unwrap();
    ///
    /// assert!(!(b.has_flag(&"#hello".to_string())));
    /// ```

    pub fn has_flag(&self, flag: &String) -> bool {
	if let Some(flags) = self.flags() {
	    flags.contains(flag)
	} else {
	    false
	}
    }

    /// Returns `true` if the item has all the given flag.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | My Title | #hello #world |\
    ///      :invalid | #valid_flag | #inva!!lid"
    /// ).unwrap();
    /// assert!(a.has_flags(vec![
    ///     "#hello".to_string(),
    ///     "#world".to_string(),
    ///     "#valid_flag".to_string(),
    /// ]));
    /// assert!(!a.has_flags(vec![
    ///     "#does".to_string(),
    ///     "#not".to_string(),
    ///     "#have".to_string(),
    /// ]));
    /// ```

    pub fn has_flags(&self, flags: Vec<String>) -> bool {
	for flag in flags {
	    if !(self.has_flag(&flag)) {
		return false;
	    }
	}
	true
    }

    /// Returns a new copy of `TagItem` with the flags inserted to the
    /// given field number, or `None` if the given field number isn't
    /// fields type.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | Field Indexing is Zero-based | #hello #world"
    /// ).unwrap();
    /// let a = a.insert_flags_to_field_no("#new #flags", 2).unwrap();
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#hello".to_string(),
    ///         "#world".to_string(),
    ///         "#new".to_string(),
    ///         "#flags".to_string(),
    ///     ]),
    ///     a.flags(),
    /// );
    /// ```
    ///
    /// ```
    /// let mut b = tag::TagItem::try_from(
    ///     "@1.0 | Field with no flags"
    /// ).unwrap();
    /// let b = b.insert_flags_to_field_no("#will-it-work", 1);
    ///
    /// assert!(b.is_none());

    pub fn insert_flags_to_field_no(
	&self,
	flags: &str,
	field_no: usize,
    ) -> Option<TagItem> {
	let mut new_tagline = String::with_capacity(
	    self.tag_line.len()
	);
	let mut iter_fields = self.fields();

	// process fields prior to field_no
	for _ in 0..field_no {
	    let Some(field) = iter_fields.next() else {
		return None
	    };

	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	    new_tagline.push('|');
	}

	// process field_no
	if let Some(field) = iter_fields.next() {
	    let Some(new_field) = field.concat_flags(flags) else {
		return None
	    };
	    new_tagline.push_str(&new_field);
	} else {
	    return None
	}

	// process fields after field_no
	while let Some(field) = iter_fields.next() {
	    new_tagline.push('|');
	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	}

	new_tagline.shrink_to_fit();

	Some ( TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	} )
    }

    /// Returns a new copy of the `TagItem` with the given flags
    /// removed, or `None` if the field isn't flags type.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | Field Indexing is Zero-based | #hello #world #new #flags"
    /// ).unwrap();
    /// let a = a.remove_flags_from_field_no("#hello #new", 2).unwrap();
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#world".to_string(),
    ///         "#flags".to_string(),
    ///     ]),
    ///     a.flags(),
    /// );
    /// ```

    pub fn remove_flags_from_field_no(
	&self,
	flags: &str,
	field_no: usize,
    ) -> Option<TagItem> {
	let mut new_tagline = String::with_capacity(
	    self.tag_line.len()
	);
	let mut iter_fields = self.fields();

	// process fields prior to field_no
	for _ in 0..field_no {
	    let Some(field) = iter_fields.next() else {
		return None
	    };
	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	    new_tagline.push('|');
	}

	// process field_no
	if let Some(field) = iter_fields.next() {
	    let Some(new_field) = field.sincat_flags(flags) else {
		return None
	    };
	    new_tagline.push_str(&new_field);
	} else {
	    return None
	}

	// process fields after field_no
	while let Some(field) = iter_fields.next() {
	    new_tagline.push('|');
	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	}

	new_tagline.shrink_to_fit();

	Some ( TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	} )
    }

    /// Returns a new copy of `TagItem` with the flag inserted.
    ///
    /// If the item doesn't have any flags field, a new one will
    /// appended, otherwise it'll use the last Flag field.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | Already with Flags | #hello #world"
    /// ).unwrap();
    /// let a = a.insert_flag("#new");
    /// let a = a.insert_flag("#flags");
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#hello".to_string(),
    ///         "#world".to_string(),
    ///         "#new".to_string(),
    ///         "#flags".to_string(),
    ///     ]),
    ///     a.flags(),
    /// );
    /// ```
    ///
    /// ```
    /// let b = tag::TagItem::try_from(
    ///     "@1.0 | No Prior Flags"
    /// ).unwrap();
    /// let b = b.insert_flag("#you_have_a_flag_now");
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#you_have_a_flag_now".to_string(),
    ///     ]),
    ///     b.flags(),
    /// );
    /// ```

    pub fn insert_flag(
	&self,
	flag: &str,
    ) -> TagItem {
	if self.has_flag(&flag.to_string()) {
	    return self.clone();
	}

	for (field_no, field) in self.fields().enumerate() {
	    let TagField::Flags(_) = field else { continue };
	    if let Some(item) = self.insert_flags_to_field_no(
		flag, field_no
	    ) {
		return item;
	    }
	}

	let mut new_tagline = self.tag_line.clone();
	new_tagline.push_str(" | ");
	new_tagline.push_str(flag);
	new_tagline.shrink_to_fit();

	TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	}
    }

    /// Returns a new copy of the `TagItem` with the flags removed.
    ///
    /// The removal is done to every flag fields of the item.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | Flags | #hello #world"
    /// ).unwrap();
    /// let a = a.remove_flags("#hello");
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#world".to_string(),
    ///     ]),
    ///     a.flags(),
    /// );
    /// ```
    ///
    /// ```
    /// let b = tag::TagItem::try_from(
    ///     "@1.0 | #many | #many #flags | #happy #flags | #cool"
    /// ).unwrap();
    /// let b = b.remove_flags("#many #happy");
    ///
    /// assert_eq!(
    ///     Some(vec![
    ///         "#flags".to_string(),
    ///         "#flags".to_string(),
    ///         "#cool".to_string(),
    ///     ]),
    ///     b.flags(),
    /// );
    /// ```

    pub fn remove_flags(
	&self,
	flags: &str
    ) -> TagItem {
	let mut new_tagline = "".to_string();
	for field in self.fields() {
	    let s = match field {
		TagField::Flags(_) => &field.sincat_flags(flags).unwrap(),
		TagField::ID(s) |
		TagField::Title(s) |
		TagField::Attribute(s) |
		TagField::Invalid(s) => s,
	    };
	    new_tagline.push_str(s);
	    new_tagline.push('|');
	}
	new_tagline.pop();
	new_tagline.shrink_to_fit();

	TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	}
    }

    /// Returns a vector of all the attributes.
    ///
    /// An attribute is represented with `(Key, value):
    /// (<Option<String>, Option<String>>)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | :src: code | :null:
    /// ").unwrap();
    /// assert_eq!(
    ///     vec![
    ///         (Some("src".to_string()), Some("code".to_string())),
    ///         (Some("null".to_string()), None),
    ///     ],
    ///     a.attributes(),
    ///);
    /// ```
    ///
    /// ```
    /// let b = tag::TagItem::try_from(
    ///     "@1.0 | Item with no attributes"
    /// ).unwrap();
    /// assert_eq!(
    ///     Vec::<(Option<String>,Option<String>)>::new(),
    ///     b.attributes(),
    /// );
    /// ```

    pub fn attributes(
	&self
    ) -> Vec<(Option<String>,Option<String>)> {
	let mut attributes: Vec<(Option<String>, Option<String>)> = Vec::new();

	for field in self.fields() {
	    let TagField::Attribute(_) = field else {continue};
	    let Some(attribute) = field.attribute_key_value() else {continue};
	    attributes.push(attribute);
	}

	attributes
    }

    /// Returns the attribute with the given key, or `None` if it's
    /// not available.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from("@1.0 | :src: code").unwrap();
    /// assert_eq!(
    ///     Some((
    ///         Some("src".to_string()),
    ///         Some("code".to_string()),
    ///     )),
    ///     a.fetch_attribute("src"),
    /// );
    /// assert_eq!(None, a.fetch_attribute("ABCD"));
    /// ```

    pub fn fetch_attribute(
	&self,
	key: &str,
    ) -> Option<(Option<String>, Option<String>)> {
	for attribute in self.attributes() {
	    if let Some(ref attribute_key) = attribute.0 {
		if attribute_key == key {
		    return Some(attribute)
		}
	    }
	}
	None
    }

    /// Returns `true` if the item has given attribute key.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from("@1.0 | :src: code").unwrap();
    /// assert!(a.has_attribute("src"));
    /// assert!(!a.has_attribute("null"));
    /// ```

    pub fn has_attribute(&self, key: &str) -> bool {
	self.fetch_attribute(key) != None
    }

    /// Returns `true` if the item matches the given attribute.
    ///
    /// If the item has multiple attributes with the same key, it will
    /// only check the first one.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from("@1.0 | :src: code | :src: new").unwrap();
    /// assert!(a.match_attribute((
    ///     Some("src"),
    ///     Some("code"),
    /// )));
    /// assert!(!a.match_attribute((
    ///     Some("attr"),
    ///     Some("doesn't have"),
    /// )));
    /// assert!(!a.match_attribute((
    ///     Some("src"),
    ///     Some("new"),
    /// )));
    /// ```

    pub fn match_attribute(
	&self,
	attribute: (Option<&str>, Option<&str>),
    ) -> bool {
	let key = match attribute.0 {
	    Some(k) => k,
	    None => "",
	};

	let Some(attribute_to_fetch) = self.fetch_attribute(key) else {
	    return false;
	};

	let attribute = (
	    if let Some(key) = attribute.0 {
		Some(key.to_string())
	    } else { None },
	    if let Some(val) = attribute.1 {
		Some(val.to_string())
	    } else { None },
	);

	attribute_to_fetch == attribute
    }

    /// Returns a new copy of `TagItem` with the attribute (key,
    /// value) set at field number, or `None` if the given field isn't
    /// attribute type.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | Program source | :src: code"
    /// ).unwrap();
    /// let a = a.set_attribute_at_field_no(
    ///     (Some("src"), Some("tag.rs")), 2
    /// ).unwrap();
    ///
    /// assert!(a.match_attribute((
    ///     Some("src"),
    ///     Some("tag.rs"),
    /// )));
    /// ```
    ///
    /// ```
    /// let b = tag::TagItem::try_from(
    ///     "@1.0 | No Attribute"
    /// ).unwrap();
    /// let b = b.set_attribute_at_field_no(
    ///     (Some("will-it-work"), Some("no")), 1
    /// );
    ///
    /// assert!(b.is_none());

    pub fn set_attribute_at_field_no (
	&self,
	attribute: (Option<&str>, Option<&str>),
	field_no: usize
    ) -> Option<TagItem> {
	let mut iter_fields = self.fields();
	let mut new_tagline = String::with_capacity(
	    self.tag_line.len()
	);

	// fields prior to field_no
	for _ in 0..field_no {
	    let Some(field) = iter_fields.next() else {
		return None
	    };
	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	    new_tagline.push('|');
	}

	// field_no field
	if let Some(field) = iter_fields.next() {
	    let TagField::Attribute(_) = field else {
		return None
	    };
	    new_tagline.push_str(" :");
	    if let Some(key) = attribute.0 {
		new_tagline.push_str(key);
	    }
	    new_tagline.push_str(": ");
	    if let Some(value) = attribute.1 {
		new_tagline.push_str(value);
	    }
	    new_tagline.push(' ');
	} else {
	    return None
	}

	// fields after field_no
	while let Some(field) = iter_fields.next() {
	    new_tagline.push('|');
	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	}

	Some ( TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	} )
    }

    /// Returns a new copy of TagItem with the attribute set.
    ///
    /// If the old item has an attribute with the given key, it'll
    /// just change the value of that attribute with the given value,
    /// otherwise it'll create a new field.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | Program source | :src: code"
    /// ).unwrap();
    /// let a = a.set_attribute((Some("src"), Some("tag.rs")));
    /// let a = a.set_attribute((Some("attr"), Some("value")));
    ///
    /// assert!(a.match_attribute((
    ///     Some("src"),
    ///     Some("tag.rs"),
    /// )));
    /// assert!(a.match_attribute((
    ///     Some("attr"),
    ///     Some("value"),
    /// )));
    /// ```

    pub fn set_attribute(
	&self,
	attribute: (Option<&str>, Option<&str>),
    ) -> TagItem {
	let attribute_key = match attribute.0 {
	    Some(k) => k,
	    None => "",
	};

	for (field_no, field) in self.fields().enumerate() {
	    println!("{:?}", field.attribute_key());
	    if field.attribute_key() == Some(attribute_key.to_string()) {
		if let Some(item) = self.set_attribute_at_field_no(
		    attribute,
		    field_no,
		) { return item };
	    }
	}

	let mut new_tagline = self.tag_line.clone();
	new_tagline.push_str(" | :");
	new_tagline.push_str(attribute_key);
	new_tagline.push(':');
	if let Some(attribute_val) = attribute.1 {
	    new_tagline.push(' ');
	    new_tagline.push_str(attribute_val);
	}
	new_tagline.shrink_to_fit();

	TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	}
    }

    /// Returns a new copy of TagItem with the attribute removed.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from("@1.0 | Program source | :src: code").unwrap();
    /// a.remove_attribute(":src:");
    /// a.remove_attribute(":does-not-have-this-one-but-ok");
    ///
    /// assert!(!a.has_attribute(":src:"));
    /// ```

    pub fn remove_attribute(
	&self,
	attribute_key: &str,
    ) -> TagItem {
	let mut new_tagline = String::with_capacity(
	    self.tag_line.len()
	);
	let mut iter_fields = self.fields();

	while let Some(field) = iter_fields.next() {
	    if field.attribute_key() == Some(attribute_key.to_string()) {
		continue
	    }
	    match field {
		TagField::ID(field_str) |
		TagField::Title(field_str) |
		TagField::Flags(field_str) |
		TagField::Attribute(field_str) |
		TagField::Invalid(field_str)  => new_tagline.push_str(field_str),
	    }
	    new_tagline.push('|');
	}

	new_tagline.pop();
	new_tagline.shrink_to_fit();

	TagItem {
	    id: self.id.clone(),
	    tag_line: new_tagline,
	}
    }
}

impl TryFrom<&str> for TagItem {
    type Error = rust_decimal::Error;

    /// Returns a TagItem from string slice, or `rust_decimal::Error`
    /// on ID parse failure.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = tag::TagItem::try_from(
    ///     "@1.0 | My Title | #hello #world"
    /// );
    /// assert!(a.is_ok());
    /// ```

    fn try_from(tag_line: &str) -> Result<TagItem, Self::Error> {
	let id_str = match tag_line.split('|').next() {
	    Some(s) => s,
	    None => "",
	};

	let id = TagID::try_from(id_str)?;

	let tag_line = String::from(tag_line);

	Ok ( TagItem { id, tag_line } )
    }
}

impl fmt::Display for TagItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	write!(f, "{}", self.tag_line)
    }
}
