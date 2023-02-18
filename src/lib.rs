use std::{
    collections::HashMap,
    fmt::{Display, Error, Formatter},
};

#[derive(Debug)]
enum XmlSchemaNode {
    /// xs:attribute
    Attribute(Attribute),
    /// xs:complexType
    ComplexType(ComplexType),
    /// xs:element
    Element(Element),
    /// xs:simpleType
    SimpleType(SimpleType),
}

#[derive(Debug)]
struct XmlSchema {
    target_namespace: Option<String>,
    element_form_default: Option<String>,
    attribute_form_default: Option<String>,
    nodes: Vec<XmlSchemaNode>,
}

#[derive(Debug)]
struct Element {
    /// The name of the element
    name: String,
    /// The datatype of the element
    datatype: Datatype,
    /// The maximum number of times the element can occur
    max_occurs: u32,
    /// The minimum number of times the element can occur
    min_occurs: u32,
}

#[derive(Debug)]
enum Datatype {
    /// A reference to a simple type
    SimpleType(String),
    /// A reference to a complex type
    ComplexType(String),
}

#[derive(Debug)]
struct Attribute {
    name: String,
    datatype: Datatype,
    default_value: Option<String>,
    fixed_value: Option<String>,
    use_option: UseOption,
}

#[derive(Debug)]
enum UseOption {
    Required,
    Optional,
}

#[derive(Debug)]
struct SimpleType {
    name: String,
    datatype: SimpleDatatype,
}

#[derive(Debug)]
enum SimpleDatatype {
    Boolean,
    Decimal,
    Double,
    Float,
    Integer,
    String,
}

#[derive(Debug)]
struct ComplexType {
    name: String,
    base_type: Option<String>,
    attributes: HashMap<String, Attribute>,
    content: ComplexContent,
    mixed_content: Option<String>,
}

#[derive(Debug)]
enum ComplexContent {
    All,
    Choice,
    Empty,
    Element,
    Group,
    Sequence,
    SimpleContent,
    Union,
    MixedContent,
    ComplexContentExtension,
}

#[derive(Debug)]
struct SimpleContent {
    datatype: SimpleDatatype,
}

#[derive(Debug)]
struct ComplexContentExtension {
    base_type: String,
    attributes: HashMap<String, Attribute>,
}

#[derive(Debug)]
struct ComplexContentRestriction {
    base_type: String,
    attributes: HashMap<String, Attribute>,
}

struct XmlSchemaParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> XmlSchemaParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    /// Parses an XML schema from the input stream.
    ///
    /// This function reads the input stream character by character,
    /// parsing different types of XML elements and adding them to an
    /// `XmlSchema` object. The function returns the `XmlSchema` object
    /// wrapped in an `Ok` variant of the `Result` type, or an error
    /// message in the form of a `String` wrapped in an `Err` variant of
    /// the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the
    /// `parse()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse();
    /// match result {
    ///     Ok(schema) => {
    ///         // Do something with the parsed schema
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing schema: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse(&mut self) -> Result<XmlSchema, String> {
        let mut schema = XmlSchema {
            target_namespace: None,
            element_form_default: None,
            attribute_form_default: None,
            nodes: Vec::new(),
        };

        while let Some(ch) = self.next_char() {
            match ch {
                '<' => {
                    let tag_name = self.parse_tag_name()?;
                    match tag_name.as_str() {
                        "xs:schema" => {
                            schema.target_namespace =
                                self.parse_optional_attr_value("targetNamespace");
                            schema.element_form_default =
                                self.parse_optional_attr_value("elementFormDefault");
                            schema.attribute_form_default =
                                self.parse_optional_attr_value("attributeFormDefault");
                        }
                        "xs:element" => {
                            let element = self.parse_element()?;
                            schema.nodes.push(XmlSchemaNode::Element(element));
                        }
                        "xs:attribute" => {
                            let attribute = self.parse_attribute()?;
                            schema.nodes.push(XmlSchemaNode::Attribute(attribute));
                        }
                        "xs:simpleType" => {
                            let simple_type = self.parse_simple_type()?;
                            schema.nodes.push(XmlSchemaNode::SimpleType(simple_type));
                        }
                        "xs:complexType" => {
                            let complex_type = self.parse_complex_type()?;
                            schema.nodes.push(XmlSchemaNode::ComplexType(complex_type));
                        }
                        _ => return Err(format!("Unexpected tag: {}", tag_name)),
                    }
                }
                _ => {
                    // Ignore non-tag characters
                }
            }
        }
        Ok(schema)
    }

    /// Returns the next character from the input stream, or `None` if
    /// there are no more characters.
    ///
    /// This function reads the input stream character by character,
    /// returning each character one at a time until the end of the
    /// stream is reached. The function returns an `Option` object that
    /// either contains the next character as a `char`, or `None` if
    /// there are no more characters to read.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the
    /// `next_char()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// while let Some(ch) = parser.next_char() {
    ///     match ch {
    ///         '<' => {
    ///             // Do something with the opening tag
    ///         }
    ///         '>' => {
    ///             // Do something with the closing tag
    ///         }
    ///         _ => {
    ///             // Do something with non-tag characters
    ///         }
    ///     }
    /// }
    /// ```
    ///
    fn next_char(&mut self) -> Option<char> {
        let ch = self.input.chars().nth(self.position);
        self.position += 1;
        ch
    }

    /// Parses the name of an XML tag from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the name of an XML tag
    /// until it encounters a whitespace character or a closing angle bracket '>'. The function returns
    /// the parsed tag name as a `String` wrapped in an `Ok` variant of the `Result` type, or an error
    /// message in the form of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_tag_name()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_tag_name();
    /// match result {
    ///     Ok(tag_name) => {
    ///         // Do something with the parsed tag name
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing tag name: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_tag_name(&mut self) -> Result<String, String> {
        let mut tag_name = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => tag_name.push(ch),
            }
        }
        Ok(tag_name)
    }

    /// Parses the value of an XML attribute from the input stream.
    ///
    /// This function reads the input stream character by character, searching for the value of an
    /// XML attribute with the given `attr_name` until it encounters a whitespace character, a closing
    /// angle bracket '>', or a double-quote character '"'. If the attribute is found and has a value,
    /// the function returns the value as a `String` wrapped in a `Some` variant of the `Option` type.
    /// If the attribute is not found or has no value, the function returns `None`.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_optional_attr_value()`
    /// function.
    /// * `attr_name` - A string slice representing the name of the XML attribute to search for.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_optional_attr_value("targetNamespace");
    /// match result {
    ///     Some(attr_value) => {
    ///         // Do something with the parsed attribute value
    ///     }
    ///     None => {
    ///         // The attribute was not found or has no value
    ///     }
    /// }
    /// ```
    ///
    fn parse_optional_attr_value(&mut self, attr_name: &str) -> Option<String> {
        let mut attr_value = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                '"' => {
                    while let Some(ch) = self.next_char() {
                        match ch {
                            '"' => break,
                            _ => attr_value.push(ch),
                        }
                    }
                    break;
                }
                _ => {}
            }
        }
        if attr_value.is_empty() {
            None
        } else {
            Some(attr_value)
        }
    }

    /// Parses an `xs:element` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the name and attributes
    /// of an `xs:element` XML element. The function returns an `Element` object containing the
    /// parsed data, wrapped in an `Ok` variant of the `Result` type, or an error message in the form
    /// of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_element()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_element();
    /// match result {
    ///     Ok(element) => {
    ///         // Do something with the parsed element
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing element: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_element(&mut self) -> Result<Element, String> {
        let mut element = Element {
            name: String::new(),
            datatype: Datatype::SimpleType(String::new()),
            max_occurs: 1,
            min_occurs: 1,
        };
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => element.name.push(ch),
            }
        }
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => {}
            }
        }
        Ok(element)
    }
    /// Parses an `xs:attribute` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the name and attributes
    /// of an `xs:attribute` XML element. The function returns an `Attribute` object containing the
    /// parsed data, wrapped in an `Ok` variant of the `Result` type, or an error message in the form
    /// of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_attribute()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_attribute();
    /// match result {
    ///     Ok(attribute) => {
    ///         // Do something with the parsed attribute
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing attribute: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_attribute(&mut self) -> Result<Attribute, String> {
        let mut attribute = Attribute {
            name: String::new(),
            datatype: Datatype::SimpleType(String::new()),
            default_value: None,
            fixed_value: None,
            use_option: UseOption::Optional,
        };
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => attribute.name.push(ch),
            }
        }
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => {}
            }
        }
        Ok(attribute)
    }
    /// Parses an `xs:simpleType` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the name and attributes
    /// of an `xs:simpleType` XML element, as well as any child elements that define the data type of
    /// the simple type. The function returns a `SimpleType` object containing the parsed data, wrapped
    /// in an `Ok` variant of the `Result` type, or an error message in the form of a `String` wrapped
    /// in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_simple_type()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_simple_type();
    /// match result {
    ///     Ok(simple_type) => {
    ///         // Do something with the parsed simple type
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing simple type: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_simple_type(&mut self) -> Result<SimpleType, String> {
        let mut simple_type = SimpleType {
            name: String::new(),
            datatype: SimpleDatatype::String,
        };
        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => simple_type.name.push(ch),
            }
        }
        while let Some(ch) = self.next_char() {
            match ch {
                '<' => break,
                _ => {}
            }
        }
        let mut datatype = SimpleDatatype::String;
        loop {
            match self.next_char() {
                Some(ch) => match ch {
                    '>' => break,
                    ' ' => {}
                    's' => {
                        let start = self.position - 1;
                        let end = start + 4;
                        if &self.input[start..end] == "type" {
                            let _ = self.parse_optional_attr_value("type");
                            match self.parse_datatype_name()?.as_str() {
                                "boolean" => datatype = SimpleDatatype::Boolean,
                                "decimal" => datatype = SimpleDatatype::Decimal,
                                "double" => datatype = SimpleDatatype::Double,
                                "float" => datatype = SimpleDatatype::Float,
                                "integer" => datatype = SimpleDatatype::Integer,
                                "string" => datatype = SimpleDatatype::String,
                                _ => return Err(format!("Unsupported datatype")),
                            }
                            break;
                        }
                    }
                    _ => {}
                },
                None => return Err(format!("Unexpected end of input")),
            }
        }
        simple_type.datatype = datatype;
        Ok(simple_type)
    }
    /// Parses the name of an XML datatype from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the name of an XML datatype
    /// until it encounters a whitespace character, a closing angle bracket '>', or a colon ':'. The
    /// function returns the parsed datatype name as a `String` wrapped in an `Ok` variant of the
    /// `Result` type, or an error message in the form of a `String` wrapped in an `Err` variant of the
    /// `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_datatype_name()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_datatype_name();
    /// match result {
    ///     Ok(datatype_name) => {
    ///         // Do something with the parsed datatype name
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing datatype name: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_datatype_name(&mut self) -> Result<String, String> {
        let mut datatype_name = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                ':' => continue,
                '>' => break,
                ' ' => break,
                _ => datatype_name.push(ch),
            }
        }
        Ok(datatype_name)
    }
    /// Parses an `xs:complexType` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the name and attributes
    /// of an `xs:complexType` XML element, as well as any child elements that define the structure
    /// and content of the complex type. The function returns a `ComplexType` object containing the
    /// parsed data, wrapped in an `Ok` variant of the `Result` type, or an error message in the form
    /// of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_complex_type()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_complex_type();
    /// match result {
    ///     Ok(complex_type) => {
    ///         // Do something with the parsed complex type
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing complex type: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_complex_type(&mut self) -> Result<ComplexType, String> {
        let mut complex_type = ComplexType {
            name: String::new(),
            base_type: None,
            attributes: HashMap::new(),
            content: ComplexContent::Empty,
            mixed_content: None,
        };

        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => complex_type.name.push(ch),
            }
        }

        while let Some(ch) = self.next_char() {
            match ch {
                '<' => {
                    let tag_name = self.parse_tag_name()?;
                    match tag_name.as_str() {
                        "xs:simpleContent" => {
                            complex_type.content = ComplexContent::SimpleContent;
                        }
                        "xs:complexContent" => {
                            complex_type.content = ComplexContent::ComplexContentExtension;
                        }
                        "xs:mixedContent" => {
                            complex_type.content = ComplexContent::MixedContent;
                            while let Some(ch) = self.next_char() {
                                if ch == '>' {
                                    break;
                                }
                            }
                            break;
                        }
                        "xs:sequence" => {
                            complex_type.content = ComplexContent::Sequence;
                        }
                        "xs:choice" => {
                            complex_type.content = ComplexContent::Choice;
                        }
                        "xs:all" => {
                            complex_type.content = ComplexContent::All;
                        }
                        _ => {
                            if tag_name == "xs:complexType" {
                                let mixed_attr = self.parse_optional_attr_value("mixed");
                                if mixed_attr == Some("true".to_string()) {
                                    complex_type.content = ComplexContent::MixedContent;
                                    while let Some(ch) = self.next_char() {
                                        if ch == '>' {
                                            break;
                                        }
                                    }
                                    break;
                                }
                            }
                            return Err(format!("Unexpected tag: {}", tag_name));
                        }
                    }
                }
                ' ' => continue,
                '>' => break,
                _ => return Err(format!("Unexpected character: {}", ch)),
            }
        }

        Ok(complex_type)
    }

    fn parse_sequence(&mut self) -> Result<Vec<XmlSchemaNode>, String> {
        let mut nodes = Vec::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '<' => {
                    let tag_name = self.parse_tag_name()?;
                    match tag_name.as_str() {
                        "xs:element" => {
                            nodes.push(XmlSchemaNode::Element(self.parse_element()?));
                        }
                        "xs:complexType" => {
                            nodes.push(XmlSchemaNode::ComplexType(self.parse_complex_type()?));
                        }
                        "xs:simpleType" => {
                            nodes.push(XmlSchemaNode::SimpleType(self.parse_simple_type()?));
                        }
                        _ => {
                            return Err(format!("Unexpected tag: {}", tag_name));
                        }
                    }
                }
                ' ' => continue,
                '>' => break,
                _ => return Err(format!("Unexpected character: {}", ch)),
            }
        }
        Ok(nodes)
    }
    /// Parses an `xs:choice` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the child elements of an
    /// `xs:choice` XML element. The function returns a vector of `XmlSchemaNode` objects containing
    /// the parsed data, wrapped in an `Ok` variant of the `Result` type, or an error message in the
    /// form of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_choice()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_choice();
    /// match result {
    ///     Ok(nodes) => {
    ///         // Do something with the parsed choice elements
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing choice elements: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_choice(&mut self) -> Result<Vec<XmlSchemaNode>, String> {
        let mut nodes = Vec::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '<' => {
                    let tag_name = self.parse_tag_name()?;
                    match tag_name.as_str() {
                        "xs:element" => {
                            nodes.push(XmlSchemaNode::Element(self.parse_element()?));
                        }
                        "xs:complexType" => {
                            nodes.push(XmlSchemaNode::ComplexType(self.parse_complex_type()?));
                        }
                        "xs:simpleType" => {
                            nodes.push(XmlSchemaNode::SimpleType(self.parse_simple_type()?));
                        }
                        _ => {
                            return Err(format!("Unexpected tag: {}", tag_name));
                        }
                    }
                }
                ' ' => continue,
                '>' => break,
                _ => return Err(format!("Unexpected character: {}", ch)),
            }
        }
        Ok(nodes)
    }
    /// Parses an `xs:all` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the child elements of an
    /// `xs:all` XML element. The function returns a vector of `XmlSchemaNode` objects containing the
    /// parsed data, wrapped in an `Ok` variant of the `Result` type, or an error message in the form
    /// of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_all()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_all();
    /// match result {
    ///     Ok(nodes) => {
    ///         // Do something with the parsed all elements
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing all elements: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_all(&mut self) -> Result<Vec<XmlSchemaNode>, String> {
        let mut nodes = Vec::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '<' => {
                    let tag_name = self.parse_tag_name()?;
                    match tag_name.as_str() {
                        "xs:element" => {
                            nodes.push(XmlSchemaNode::Element(self.parse_element()?));
                        }
                        "xs:complexType" => {
                            nodes.push(XmlSchemaNode::ComplexType(self.parse_complex_type()?));
                        }
                        "xs:simpleType" => {
                            nodes.push(XmlSchemaNode::SimpleType(self.parse_simple_type()?));
                        }
                        _ => {
                            return Err(format!("Unexpected tag: {}", tag_name));
                        }
                    }
                }
                ' ' => continue,
                '>' => break,
                _ => return Err(format!("Unexpected character: {}", ch)),
            }
        }
        Ok(nodes)
    }
    /// Parses an `xs:simpleContent` XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the child elements of an
    /// `xs:simpleContent` XML element. The function returns a `SimpleContent` object containing the
    /// parsed data, wrapped in an `Ok` variant of the `Result` type, or an error message in the form
    /// of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_simple_content()` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_simple_content();
    /// match result {
    ///     Ok(simple_content) => {
    ///         // Do something with the parsed simple content
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing simple content: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_simple_content(&mut self) -> Result<SimpleContent, String> {
        let mut simple_content = SimpleContent {
            datatype: SimpleDatatype::String,
        };

        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => {}
            }
        }

        let mut datatype = SimpleDatatype::String;
        loop {
            match self.next_char() {
                Some(ch) => match ch {
                    '>' => break,
                    ' ' => {}
                    's' => {
                        let start = self.position - 1;
                        let end = start + 4;
                        if &self.input[start..end] == "type" {
                            let _ = self.parse_optional_attr_value("type");
                            match self.parse_datatype_name()?.as_str() {
                                "boolean" => datatype = SimpleDatatype::Boolean,
                                "decimal" => datatype = SimpleDatatype::Decimal,
                                "double" => datatype = SimpleDatatype::Double,
                                "float" => datatype = SimpleDatatype::Float,
                                "integer" => datatype = SimpleDatatype::Integer,
                                "string" => datatype = SimpleDatatype::String,
                                _ => return Err(format!("Unsupported datatype")),
                            }
                            break;
                        }
                    }
                    _ => {}
                },
                None => return Err(format!("Unexpected end of input")),
            }
        }
        simple_content.datatype = datatype;

        while let Some(ch) = self.next_char() {
            match ch {
                '<' => break,
                _ => {}
            }
        }

        Ok(simple_content)
    }
    /// Parses an `xs:complexContent` extension XML element from the input stream.
    ///
    /// This function reads the input stream character by character, parsing the child elements of an
    /// `xs:complexContent` extension XML element. The function returns a `ComplexContentExtension`
    /// object containing the parsed data, wrapped in an `Ok` variant of the `Result` type, or an error
    /// message in the form of a `String` wrapped in an `Err` variant of the `Result` type.
    ///
    /// # Arguments
    ///
    /// * `self` - A mutable reference to an object that implements the `parse_complex_content_extension()`
    ///            function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut parser = XmlSchemaParser::new(input);
    /// let result = parser.parse_complex_content_extension();
    /// match result {
    ///     Ok(complex_content_extension) => {
    ///         // Do something with the parsed complex content extension
    ///     }
    ///     Err(message) => {
    ///         eprintln!("Error parsing complex content extension: {}", message);
    ///     }
    /// }
    /// ```
    ///
    fn parse_complex_content_extension(&mut self) -> Result<ComplexContentExtension, String> {
        let mut complex_content_extension = ComplexContentExtension {
            base_type: String::new(),
            attributes: HashMap::new(),
        };

        while let Some(ch) = self.next_char() {
            match ch {
                '>' => break,
                ' ' => break,
                _ => {}
            }
        }

        while let Some(ch) = self.next_char() {
            match ch {
                ' ' => continue,
                'b' => {
                    let start = self.position - 1;
                    let end = start + 8;
                    if &self.input[start..end] == "base=\"xs" {
                        let base_type = self.parse_datatype_name()?;
                        complex_content_extension.base_type = base_type;
                        break;
                    } else {
                        return Err(format!("Unexpected character: {}", ch));
                    }
                }
                _ => return Err(format!("Unexpected character: {}", ch)),
            }
        }
        Ok(complex_content_extension)
    }
}

impl Display for SimpleDatatype {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            SimpleDatatype::Boolean => write!(f, "boolean"),
            SimpleDatatype::Decimal => write!(f, "decimal"),
            SimpleDatatype::Double => write!(f, "double"),
            SimpleDatatype::Float => write!(f, "float"),
            SimpleDatatype::Integer => write!(f, "integer"),
            SimpleDatatype::String => write!(f, "string"),
        }
    }
}
