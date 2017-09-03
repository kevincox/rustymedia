use std::io::Write;
use serde;
use std;

mod error {
	use serde;
	use std;
	
	error_chain!{
		errors {
			Unsupported(method: &'static str)
		}
		
		foreign_links {
			Io(::std::io::Error);
			Xml(::serde_xml_rs::Error);
		}
	}
	
	impl serde::ser::Error for Error {
		fn custom<T: std::fmt::Display>(msg: T) -> Self {
			msg.to_string().into()
		}
	}
}
pub use xml::error::{Error, ErrorKind, Result};

#[derive(Debug,Serialize)]
#[serde(rename="||KXML body node||")]
pub struct Body<T>(pub T);

pub fn serialize<W: Write, S: serde::Serialize>(out: W, val: S) -> Result<()> {
	val.serialize(&mut Serializer::new(out))
}

fn check_valid_name(_name: &str) -> Result<()> { Ok(()) }
fn check_valid_attr(_name: &str) -> Result<()> { Ok(()) }

pub struct Serializer<W> {
	writer: W,
}

impl<W: std::io::Write> Serializer<W>
{
	pub fn new(writer: W) -> Self {
		Self { writer: writer }
	}
}

impl<'a, W: Write> serde::ser::Serializer for &'a mut Serializer<W> {
	type Ok = ();
	type Error = Error;
	
	type SerializeSeq = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeStruct = Struct<'a, W>;
	type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
	
	fn serialize_bool(mut self, v: bool) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_i8(mut self, v: i8) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_i16(mut self, v: i16) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_i32(mut self, v: i32) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_i64(mut self, v: i64) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_u8(mut self, v: u8) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_u16(mut self, v: u16) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_u32(mut self, v: u32) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_u64(mut self, v: u64) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_f32(mut self, v: f32) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_f64(mut self, v: f64) -> Result<Self::Ok> {
		write!(self.writer, "{}", v)?; Ok(())
	}
	
	fn serialize_char(mut self, c: char) -> Result<Self::Ok> {
		match c {
			'"' => write!(self.writer, "&quot;")?,
			'<' => write!(self.writer, "&lt;")?,
			'&' => write!(self.writer, "&amp;")?,
			c => write!(self.writer, "{}", c)?,
		}
		Ok(())
	}
	
	fn serialize_str(mut self, value: &str) -> Result<Self::Ok> {
		for c in value.chars() {
			self.serialize_char(c)?
		}
		Ok(())
	}
	
	fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok> {
		Err(ErrorKind::Unsupported("serialize_bytes").into())
	}
	
	fn serialize_none(self) -> Result<Self::Ok> {
		Ok(())
	}
	
	fn serialize_some<T: ?Sized + serde::Serialize>(self, value: &T) -> Result<Self::Ok> {
		value.serialize(self)
	}
	
	fn serialize_unit(self) -> Result<Self::Ok> {
		Ok(())
	}
	
	fn serialize_unit_struct(mut self, name: &'static str) -> Result<Self::Ok> {
		check_valid_name(name)?;
		write!(self.writer, "<{}/>", name)?;
		Ok(())
	}
	
	fn serialize_unit_variant(self,
		_name: &'static str, _variant_index: u32, _variant: &'static str)
		-> Result<Self::Ok>
	{
		Err(ErrorKind::Unsupported("serialize_unit_variant").into())
	}
	
	fn serialize_newtype_struct<T: ?Sized + serde::Serialize>
		(self, _name: &'static str, value: &T)
		-> Result<Self::Ok>
	{
		value.serialize(&mut *self)?;
		Ok(())
	}
	
	fn serialize_newtype_variant<T: ?Sized + serde::Serialize>
		(mut self, name: &'static str, _variant_index: u32, variant: &'static str, value: &T)
		-> Result<Self::Ok>
	{
		check_valid_name(name)?;
		write!(self.writer, "<{}>", variant)?;
		value.serialize(&mut *self)?;
		write!(self.writer, "</{}>", variant)?;
		Ok(())
	}
	
	fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
		Err(ErrorKind::Unsupported("serialize_seq").into())
	}
	
	fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
		Err(ErrorKind::Unsupported("serialize_tuple").into())
	}
	
	fn serialize_tuple_struct(self, _name: &'static str, _len: usize)
		-> Result<Self::SerializeTupleStruct>
	{
		Err(ErrorKind::Unsupported("serialize_tuple_struct").into())
	}
	
	fn serialize_tuple_variant(self,
		_name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize)
		-> Result<Self::SerializeTupleVariant>
	{
		Err(ErrorKind::Unsupported("serialize_tuple_variant").into())
	}
	
	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
		Err(ErrorKind::Unsupported("serialize_map").into())
	}
	
	fn serialize_struct(mut self, name: &'static str, _len: usize)
		-> Result<Self::SerializeStruct>
	{
		check_valid_name(name)?;
		write!(self.writer, "<{}", name)?;
		Ok(Struct{
			parent: self,
			name: name,
			body: false,
		})
	}
	
	fn serialize_struct_variant(self,
		_name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize)
		-> Result<Self::SerializeStructVariant>
	{
		Err(ErrorKind::Unsupported("serialize_struct_variant").into())
	}
}

pub struct Seq<'a, 'b: 'a, W: 'b> {
	parent: SubSerializer<'a, 'b, W>,
	name: &'static str,
}

impl<'a, 'b, W: Write> serde::ser::SerializeSeq for Seq<'a, 'b, W> {
	type Ok = ();
	type Error = Error;
	
	fn serialize_element<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
		value.serialize(SeqSubSerializer {
			name: self.name,
			parent: self,
		})?;
		Ok(())
	}
	
	fn end(self) -> Result<()> {
		Ok(())
	}
}

pub struct SeqSubSerializer<'a, 'b: 'a, 'c: 'b, W: 'c> {
	parent: &'a mut Seq<'b, 'c, W>,
	name: &'static str,
}

impl<'a, 'b, 'c, W: Write> SeqSubSerializer<'a, 'b, 'c, W> {
	fn wrapped<F: FnOnce(&mut Serializer<W>)->Result<()>>(&mut self, f: F) -> Result<()> {
		check_valid_name(self.name)?;
		
		write!(self.parent.parent.parent.parent.writer, "<{}>", self.name)?;
		let r = f(self.root());
		write!(self.parent.parent.parent.parent.writer, "</{}>", self.name)?;
		r
	}
	
	fn root(&mut self) -> &mut Serializer<W> {
		self.parent.parent.parent.parent
	}
	
	fn writer(&mut self) -> &mut W {
		&mut self.root().writer
	}
}

impl<'a, 'b, 'c, W: Write> serde::ser::Serializer for SeqSubSerializer<'a, 'b, 'c, W> {
	type Ok = ();
	type Error = Error;
	
	type SerializeSeq = Seq<'a, 'b, W>;
	type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeStruct = Struct<'a, W>;
	type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
	
	fn serialize_bool(mut self, v: bool) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_bool(v))
	}
	
	fn serialize_i8(mut self, v: i8) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_i8(v))
	}
	
	fn serialize_i16(mut self, v: i16) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_i16(v))
	}
	
	fn serialize_i32(mut self, v: i32) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_i32(v))
	}
	
	fn serialize_i64(mut self, v: i64) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_i64(v))
	}
	
	fn serialize_u8(mut self, v: u8) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_u8(v))
	}
	
	fn serialize_u16(mut self, v: u16) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_u16(v))
	}
	
	fn serialize_u32(mut self, v: u32) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_u32(v))
	}
	
	fn serialize_u64(mut self, v: u64) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_u64(v))
	}
	
	fn serialize_f32(mut self, v: f32) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_f32(v))
	}
	
	fn serialize_f64(mut self, v: f64) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_f64(v))
	}
	
	fn serialize_char(mut self, c: char) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_char(c))
	}
	
	fn serialize_str(mut self, value: &str) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_str(value))
	}
	
	fn serialize_bytes(mut self, value: &[u8]) -> Result<Self::Ok> {
		self.wrapped(|p| p.serialize_bytes(value))
	}
	
	fn serialize_none(self) -> Result<Self::Ok> {
		Ok(())
	}
	
	fn serialize_some<T: ?Sized + serde::Serialize>(self, _value: &T) -> Result<Self::Ok> {
		Err(ErrorKind::Unsupported("serialize_some inside sequence").into())
	}
	
	fn serialize_unit(self) -> Result<Self::Ok> {
		Ok(())
	}
	
	fn serialize_unit_struct(mut self, name: &'static str) -> Result<Self::Ok> {
		self.root().serialize_unit_struct(name)
	}
	
	fn serialize_unit_variant(mut self,
		name: &'static str, variant_index: u32, variant: &'static str)
		-> Result<Self::Ok>
	{
		self.root().serialize_unit_variant(name, variant_index, variant)
	}
	
	fn serialize_newtype_struct<T: ?Sized + serde::Serialize>
		(mut self, name: &'static str, value: &T)
		-> Result<Self::Ok>
	{
		self.root().serialize_newtype_struct(name, value)
	}
	
	fn serialize_newtype_variant<T: ?Sized + serde::Serialize>
		(mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T)
		-> Result<Self::Ok>
	{
		self.root().serialize_newtype_variant(name, variant_index, variant, value)
	}
	
	fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
		Err(ErrorKind::Unsupported("serialize_seq").into())
	}
	
	fn serialize_tuple(mut self, len: usize) -> Result<Self::SerializeTuple> {
		self.root().serialize_tuple(len)
	}
	
	fn serialize_tuple_struct(mut self,
		name: &'static str, len: usize)
		-> Result<Self::SerializeTupleStruct>
	{
		self.root().serialize_tuple_struct(name, len)
	}
	
	fn serialize_tuple_variant(mut self,
		name: &'static str,
		variant_index: u32, variant: &'static str,
		len: usize)
		-> Result<Self::SerializeTupleVariant>
	{
		self.root().serialize_tuple_variant(name, variant_index, variant, len)
	}
	
	fn serialize_map(mut self, len: Option<usize>) -> Result<Self::SerializeMap> {
		self.root().serialize_map(len)
	}
	
	fn serialize_struct(mut self, name: &'static str, _len: usize)
		-> Result<Self::SerializeStruct>
	{
		check_valid_name(name)?;
		write!(self.writer(), "<{}", name)?;
		Ok(Struct {
			parent: self.parent.parent.parent.parent,
			name: name,
			body: false,
		})
	}
	
	fn serialize_struct_variant(mut self,
		name: &'static str, variant_index: u32, variant: &'static str, len: usize)
		-> Result<Self::SerializeStructVariant>
	{
		self.root().serialize_struct_variant(name, variant_index, variant, len)
	}
}

pub struct Struct<'a, W: 'a>
{
	parent: &'a mut Serializer<W>,
	name: &'static str,
	body: bool,
}

impl<'a, W: std::io::Write> Struct<'a, W> {
	fn enter_body(&mut self) -> Result<()> {
		if !self.body {
			self.body = true;
			write!(self.parent.writer, ">")?;
		}
		Ok(())
	}
}

impl<'a, W: Write> serde::ser::SerializeStruct for Struct<'a, W> {
	type Ok = ();
	type Error = Error;
	
	fn serialize_field<T: ?Sized + serde::Serialize>
		(&mut self, key: &'static str, value: &T) -> Result<()>
	{
		value.serialize(SubSerializer{parent: self, name: key})?;
		Ok(())
	}
	
	fn end(mut self) -> Result<Self::Ok> {
		if self.body {
			write!(self.parent.writer, "</{}>", self.name)?;
		} else {
			write!(self.parent.writer, "/>")?;
		}
		Ok(())
	}
}

pub struct SubSerializer<'a, 'b: 'a, W: 'b> {
	parent: &'a mut Struct<'b, W>,
	name: &'static str,
}

impl<'a, 'b, W: Write> SubSerializer<'a, 'b, W> {
	fn attr<F: FnOnce(&mut Serializer<W>)->Result<()>>(&mut self, f: F) -> Result<()> {
		check_valid_attr(self.name)?;
		
		if self.parent.body { return self.child(f) }
		
		write!(self.parent.parent.writer, " {}=\"", self.name)?;
		let r = f(self.parent.parent);
		write!(self.parent.parent.writer, "\"")?;
		r
	}
	
	fn child<F: FnOnce(&mut Serializer<W>)->Result<()>>(&mut self, f: F) -> Result<()> {
		check_valid_name(self.name)?;
		
		self.parent.enter_body()?;
		write!(self.parent.parent.writer, "<{}>", self.name)?;
		let r = f(self.parent.parent);
		write!(self.parent.parent.writer, "</{}>", self.name)?;
		r
	}
}

impl<'a, 'b, W: Write> serde::ser::Serializer for SubSerializer<'a, 'b, W> {
	type Ok = ();
	type Error = Error;
	
	type SerializeSeq = Seq<'a, 'b, W>;
	type SerializeTuple = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTupleStruct = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeTupleVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeMap = serde::ser::Impossible<Self::Ok, Self::Error>;
	type SerializeStruct = Struct<'a, W>;
	type SerializeStructVariant = serde::ser::Impossible<Self::Ok, Self::Error>;
	
	fn serialize_bool(mut self, v: bool) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_bool(v))
	}
	
	fn serialize_i8(mut self, v: i8) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_i8(v))
	}
	
	fn serialize_i16(mut self, v: i16) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_i16(v))
	}
	
	fn serialize_i32(mut self, v: i32) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_i32(v))
	}
	
	fn serialize_i64(mut self, v: i64) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_i64(v))
	}
	
	fn serialize_u8(mut self, v: u8) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_u8(v))
	}
	
	fn serialize_u16(mut self, v: u16) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_u16(v))
	}
	
	fn serialize_u32(mut self, v: u32) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_u32(v))
	}
	
	fn serialize_u64(mut self, v: u64) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_u64(v))
	}
	
	fn serialize_f32(mut self, v: f32) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_f32(v))
	}
	
	fn serialize_f64(mut self, v: f64) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_f64(v))
	}
	
	fn serialize_char(mut self, c: char) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_char(c))
	}
	
	fn serialize_str(mut self, value: &str) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_str(value))
	}
	
	fn serialize_bytes(mut self, value: &[u8]) -> Result<Self::Ok> {
		self.child(|p| p.serialize_bytes(value))
	}
	
	fn serialize_none(self) -> Result<Self::Ok> {
		Ok(())
	}
	
	fn serialize_some<T: ?Sized + serde::Serialize>(mut self, value: &T) -> Result<Self::Ok> {
		self.attr(|p| p.serialize_some(value))
	}
	
	fn serialize_unit(self) -> Result<Self::Ok> {
		Ok(())
	}
	
	fn serialize_unit_struct(mut self, name: &'static str) -> Result<Self::Ok> {
		self.child(|p| p.serialize_unit_struct(name))
	}
	
	fn serialize_unit_variant(mut self,
		name: &'static str, variant_index: u32, variant: &'static str)
		-> Result<Self::Ok>
	{
		self.attr(|p| p.serialize_unit_variant(name, variant_index, variant))
	}
	
	fn serialize_newtype_struct<T: ?Sized + serde::Serialize>
		(mut self, name: &'static str, value: &T)
		-> Result<Self::Ok>
	{
		if !self.parent.body && name == "||KXML body node||" {
			self.parent.enter_body()?;
			value.serialize(&mut *self.parent.parent)
		} else {
			self.child(|p| value.serialize(p))
		}
	}
	
	fn serialize_newtype_variant<T: ?Sized + serde::Serialize>
		(mut self, name: &'static str, variant_index: u32, variant: &'static str, value: &T)
		-> Result<Self::Ok>
	{
		self.attr(|p| p.serialize_newtype_variant(name, variant_index, variant, value))
	}
	
	fn serialize_seq(mut self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
		self.parent.enter_body()?;
		Ok(Seq{
			name: self.name,
			parent: self,
		})
	}
	
	fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
		self.parent.enter_body()?;
		self.parent.parent.serialize_tuple(len)
	}
	
	fn serialize_tuple_struct(self,
		name: &'static str, len: usize)
		-> Result<Self::SerializeTupleStruct>
	{
		self.parent.enter_body()?;
		self.parent.parent.serialize_tuple_struct(name, len)
	}
	
	fn serialize_tuple_variant(self,
		name: &'static str,
		variant_index: u32, variant: &'static str,
		len: usize)
		-> Result<Self::SerializeTupleVariant>
	{
		self.parent.enter_body()?;
		self.parent.parent.serialize_tuple_variant(name, variant_index, variant, len)
	}
	
	fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
		self.parent.enter_body()?;
		self.parent.parent.serialize_map(len)
	}
	
	fn serialize_struct(mut self, name: &'static str, len: usize)
		-> Result<Self::SerializeStruct>
	{
		self.parent.enter_body()?;
		self.parent.parent.serialize_struct(name, len)
	}
	
	fn serialize_struct_variant(self,
		name: &'static str, variant_index: u32, variant: &'static str, len: usize)
		-> Result<Self::SerializeStructVariant>
	{
		self.parent.enter_body()?;
		self.parent.parent.serialize_struct_variant(name, variant_index, variant, len)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[derive(Serialize)]
	struct Simple<T> {
		a: String,
		b: i64,
		val: Body<T>,
	}
	
	fn to_xml<T: serde::Serialize>(obj: T) -> String {
		let mut buf = Vec::new();
		serialize(&mut buf, obj).unwrap();
		String::from_utf8(buf).unwrap()
	}
	
	#[test]
	fn test_simple() {
		let obj = Simple{
			a: "foo".to_string(),
			b: 17,
			val: Body("contents".to_string()),
		};
		assert_eq!(to_xml(obj), r#"<Simple a="foo" b="17">contents</Simple>"#);
	}
	
	#[derive(Serialize)]
	struct NewType(String);
	
	#[derive(Serialize)]
	struct NoAttr {
		new_type: NewType,
	}
	
	#[test]
	fn test_new_type() {
		let obj = NoAttr{
			new_type: NewType("foobar".to_string()),
		};
		assert_eq!(to_xml(obj), r#"<NoAttr><new_type>foobar</new_type></NoAttr>"#);
	}
	
	#[derive(Serialize)]
	struct Seq {
		a: Vec<u32>,
		n: Vec<u32>,
	}
	
	#[test]
	fn test_seq() {
		let obj = Seq{
			a: vec![],
			n: vec![1, 2, 3],
		};
		assert_eq!(to_xml(obj), r#"<Seq><n>1</n><n>2</n><n>3</n></Seq>"#);
	}
	
	#[derive(Serialize)]
	struct SeqNT {
		a: Vec<Simple<i32>>,
		n: Vec<Simple<i32>>,
	}
	
	#[test]
	fn test_seq_type() {
		let obj = SeqNT{
			a: vec![],
			n: vec![
				Simple{a: "foo".to_string(), b: 42, val: Body(3)},
				Simple{a: "bar".to_string(), b: 2, val: Body(14)},
			],
		};
		assert_eq!(to_xml(obj), r#"<SeqNT><Simple a="foo" b="42">3</Simple><Simple a="bar" b="2">14</Simple></SeqNT>"#);
	}
	
	#[derive(Serialize)]
	struct Complex<T> {
		a: String,
		b: Simple<T>,
		c: String,
	}
	
	#[test]
	fn test_complex() {
		let obj = Complex{
			a: "a b c".to_string(),
			b: Simple{
				a: r#"<script>alert("Hi & Bye!")</script>"#.to_string(),
				b: 42,
				val: Body(Simple{
					a: "such nesting".to_string(),
					b: 9001,
					val: Body(3918),
				}),
			},
			c: "finally".to_string(),
		};
		assert_eq!(to_xml(obj), r#"<Complex a="a b c"><Simple a="&lt;script>alert(&quot;Hi &amp; Bye!&quot;)&lt;/script>" b="42"><Simple a="such nesting" b="9001">3918</Simple></Simple><c>finally</c></Complex>"#);
	}
}
