
use rustc_hash::FxHashMap;

/// Unique identifier for a registered shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub u32);

impl ShapeId {
    /// Sentinel value meaning "no shape / unknown shape"
    pub const NONE: ShapeId = ShapeId(u32::MAX);
}

/// Describes the "shape" of a value — what properties and methods it has.
#[derive(Debug, Clone)]
pub struct ObjectShape {
    /// Properties available on this type
    pub properties: FxHashMap<String, PropertyShape>,
    /// If the shape is callable (e.g., a function)
    pub call_signature: Option<FunctionSignature>,
    /// If the shape is constructable (e.g., a class)
    pub construct_signature: Option<FunctionSignature>,
}

impl ObjectShape {
    pub fn new() -> Self {
        Self { properties: FxHashMap::default(), call_signature: None, construct_signature: None }
    }
}

impl Default for ObjectShape {
    fn default() -> Self {
        Self::new()
    }
}

/// Describes the shape of a property on an object.
#[derive(Debug, Clone)]
pub struct PropertyShape {
    /// The shape of the property's value
    pub value_shape: ShapeId,
    /// Whether the property is writable
    pub writable: bool,
}

/// How a function call affects its arguments and return value.
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Effect on each parameter
    pub params: Vec<ParamEffect>,
    /// Shape of the return value
    pub return_shape: ShapeId,
    /// What kind of call this is
    pub call_kind: CallKind,
    /// Whether arguments are guaranteed not to be aliased by the function
    pub no_alias: bool,
}

/// How a function parameter is used by the function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamEffect {
    /// Parameter is only read
    Read,
    /// Parameter is mutated
    Mutate,
    /// Parameter is frozen (becomes immutable)
    Freeze,
    /// Parameter is captured (stored for later use)
    Capture,
    /// Parameter is conditionally mutated
    ConditionalMutate,
}

/// Classifies the kind of function call for effect analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallKind {
    /// Normal function call
    Normal,
    /// React hook call
    Hook,
    /// Known impure function (e.g., Math.random, Date.now)
    Impure,
    /// Known pure function (no side effects)
    Pure,
}

/// Registry of all known object shapes.
/// Shapes are stored in a flat Vec indexed by ShapeId.
#[derive(Debug)]
pub struct ShapeRegistry {
    shapes: Vec<ObjectShape>,
}

impl ShapeRegistry {
    pub fn new() -> Self {
        Self { shapes: Vec::new() }
    }

    /// Register a new shape and return its ID.
    pub fn register_shape(&mut self, shape: ObjectShape) -> ShapeId {
        let id = ShapeId(self.shapes.len() as u32);
        self.shapes.push(shape);
        id
    }

    /// Get a shape by its ID.
    pub fn get_shape(&self, id: ShapeId) -> Option<&ObjectShape> {
        if id == ShapeId::NONE {
            return None;
        }
        self.shapes.get(id.0 as usize)
    }

    /// Get a mutable reference to a shape by its ID.
    pub fn get_shape_mut(&mut self, id: ShapeId) -> Option<&mut ObjectShape> {
        if id == ShapeId::NONE {
            return None;
        }
        self.shapes.get_mut(id.0 as usize)
    }

    /// Look up a property shape on a given shape.
    pub fn get_property_shape(&self, shape_id: ShapeId, property: &str) -> Option<&PropertyShape> {
        self.get_shape(shape_id).and_then(|shape| shape.properties.get(property))
    }

    /// Get the call signature for a shape, if callable.
    pub fn get_call_signature(&self, shape_id: ShapeId) -> Option<&FunctionSignature> {
        self.get_shape(shape_id).and_then(|shape| shape.call_signature.as_ref())
    }

    /// Number of registered shapes.
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
}

impl Default for ShapeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
