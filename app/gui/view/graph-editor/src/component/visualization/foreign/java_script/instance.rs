//! This module contains functionality that allows the usage of JavaScript to define visualizations.
//!
//! The `Instance` defines a generic way to wrap JS function calls and allow interaction with
//! JS code and the visualization system.
//!
//! An `Instance` can be created via `Instance::from_object` where the a JS object is provided that
//! fullfills the spec described in `java_script/definition.rs

use crate::component::visualization::*;
use crate::prelude::*;
use ensogl::system::web::traits::*;

use crate::component::visualization;
use crate::component::visualization::instance::PreprocessorConfiguration;
use crate::component::visualization::java_script;
use crate::component::visualization::java_script::binding::JsConsArgs;
use crate::component::visualization::java_script::method;

use core::result;
use enso_frp as frp;
use ensogl::application::Application;
use ensogl::data::color;
use ensogl::display;
use ensogl::display::shape::StyleWatch;
use ensogl::display::DomScene;
use ensogl::display::DomSymbol;
use ensogl::display::Scene;
use ensogl::system::web;
use ensogl::system::web::JsValue;
use std::fmt::Formatter;



// ==============
// === Errors ===
// ==============

/// Errors that can occur when transforming JS source to a visualization.
#[derive(Clone, Debug)]
#[allow(missing_docs)]
pub enum Error {
    /// The provided `JsValue` was expected to be of type `object`, but was not.
    ValueIsNotAnObject { object: JsValue },
    /// The object was expected to have the named property but does not.
    PropertyNotFoundOnObject { object: JsValue, property: String },
    /// An error occurred on the javascript side when calling the class constructor.
    ConstructorError { js_error: JsValue },
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::ValueIsNotAnObject { object } => f.write_fmt(format_args!(
                "JsValue was expected to be of type `object`, but was not: {object:?}"
            )),
            Error::PropertyNotFoundOnObject { object, property } => f.write_fmt(format_args!(
                "Object was expected to have property {property:?} but has not: {object:?}"
            )),
            Error::ConstructorError { js_error } =>
                f.write_fmt(format_args!("Error while constructing object: {js_error:?}")),
        }
    }
}

impl std::error::Error for Error {}

/// Internal helper type to propagate results that can fail due to `JsVisualizationError`s.
pub type Result<T> = result::Result<T, Error>;



// =====================
// === InstanceModel ===
// =====================

/// Helper type for the callback used to set the preprocessor code.
pub trait PreprocessorCallback = Fn(PreprocessorConfiguration);

/// Internal helper type to store the preprocessor callback.
type PreprocessorCallbackCell = Rc<RefCell<Option<Box<dyn PreprocessorCallback>>>>;

/// `JsVisualizationGeneric` allows the use of arbitrary javascript to create visualizations. It
/// takes function definitions as strings and proved those functions with data.
#[derive(Clone, CloneRef, Derivative)]
#[derivative(Debug)]
#[allow(missing_docs)]
pub struct InstanceModel {
    pub root_node:       DomSymbol,
    on_data_received:    Rc<Option<web::Function>>,
    set_size:            Rc<Option<web::Function>>,
    #[derivative(Debug = "ignore")]
    object:              Rc<java_script::binding::Visualization>,
    #[derivative(Debug = "ignore")]
    preprocessor_change: PreprocessorCallbackCell,
    scene:               Scene,
}

impl InstanceModel {
    fn get_background_color(scene: &Scene) -> color::Rgba {
        let styles = StyleWatch::new(&scene.style_sheet);
        styles.get_color(ensogl_hardcoded_theme::graph_editor::visualization::background)
    }

    fn create_root(scene: &Scene) -> result::Result<DomSymbol, Error> {
        let div = web::document.create_div_or_panic();
        let root_node = DomSymbol::new(&div);
        root_node
            .dom()
            .set_attribute("class", "visualization")
            .map_err(|js_error| Error::ConstructorError { js_error })?;

        let bg_color = Self::get_background_color(scene);
        let bg_red = bg_color.red * 255.0;
        let bg_green = bg_color.green * 255.0;
        let bg_blue = bg_color.blue * 255.0;
        let bg_hex = format!("rgba({},{},{},{})", bg_red, bg_green, bg_blue, bg_color.alpha);
        root_node.dom().set_style_or_warn("background", bg_hex);
        Ok(root_node)
    }

    /// We need to provide a closure to the Visualization on the JS side, which we then later
    /// can hook up to the FRP. Here we create a `PreprocessorCallbackCell`, which can hold a
    /// closure, and a `PreprocessorCallback` which holds a weak reference to the closure inside of
    /// the `PreprocessorCallbackCell`. This allows us to pass the `PreprocessorCallback` to the
    /// javascript code, and call from there the closure stored in the `PreprocessorCallbackCell`.
    /// We will later on set the closure inside of the `PreprocessorCallbackCell` to emit an FRP
    /// event.
    fn preprocessor_change_callback() -> (PreprocessorCallbackCell, impl PreprocessorCallback) {
        let closure_cell = PreprocessorCallbackCell::default();
        let weak_closure_cell = Rc::downgrade(&closure_cell);
        let closure = move |preprocessor_config| {
            if let Some(callback) = weak_closure_cell.upgrade() {
                callback.borrow().map_ref(|f| f(preprocessor_config));
            }
        };
        (closure_cell, closure)
    }

    #[cfg(target_arch = "wasm32")]
    fn instantiate_class_with_args(
        class: &JsValue,
        args: JsConsArgs,
    ) -> result::Result<java_script::binding::Visualization, Error> {
        let js_new = web::Function::new_with_args_fixed("cls,arg", "return new cls(arg)").unwrap();
        let context = JsValue::NULL;
        let object = js_new
            .call2(&context, class, &args.into())
            .map_err(|js_error| Error::ConstructorError { js_error })?;
        if !object.is_object() {
            return Err(Error::ValueIsNotAnObject { object });
        }
        let object: java_script::binding::Visualization = object.into();
        Ok(object)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn instantiate_class_with_args(
        _class: &JsValue,
        _args: JsConsArgs,
    ) -> result::Result<java_script::binding::Visualization, Error> {
        Ok(java_script::binding::Visualization::new())
    }

    /// Tries to create a InstanceModel from the given visualization class.
    pub fn from_class(class: &JsValue, scene: &Scene) -> result::Result<Self, Error> {
        let root_node = Self::create_root(scene)?;
        let (preprocessor_change, closure) = Self::preprocessor_change_callback();
        let styles = StyleWatch::new(&scene.style_sheet);
        let init_data = JsConsArgs::new(root_node.clone_ref(), styles, closure);
        let object = Self::instantiate_class_with_args(class, init_data)?;
        let on_data_received = get_method(&object, method::ON_DATA_RECEIVED).ok();
        let on_data_received = Rc::new(on_data_received);
        let set_size = get_method(&object, method::SET_SIZE).ok();
        let set_size = Rc::new(set_size);
        let object = Rc::new(object);
        let scene = scene.clone_ref();
        Ok(InstanceModel {
            root_node,
            on_data_received,
            set_size,
            object,
            preprocessor_change,
            scene,
        })
    }

    /// Hooks the root node into the given scene.
    ///
    /// MUST be called to make this visualization visible.
    pub fn set_dom_layer(&self, scene: &DomScene) {
        scene.manage(&self.root_node);
    }

    #[cfg(target_arch = "wasm32")]
    fn set_size(&self, size: Vector2) {
        let data_json = serde_wasm_bindgen::to_value(&size).unwrap();
        let _ = self.try_call1(&self.set_size, &data_json);
        self.root_node.set_dom_size(size);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_size(&self, _size: Vector2) {}

    #[profile(Debug)]
    #[cfg(target_arch = "wasm32")]
    fn receive_data(&self, data: &Data) -> result::Result<(), DataError> {
        let data_json = data.as_json()?.raw();
        let data_js = js_sys::JSON::parse(data_json).map_err(|_| DataError::InvalidDataType)?;
        self.try_call1(&self.on_data_received, &data_js)
            .map_err(|_| DataError::InternalComputationError)?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn receive_data(&self, _data: &Data) -> result::Result<(), DataError> {
        Ok(())
    }

    /// Prompt visualization JS object to emit preprocessor change with its currently desired state.
    pub fn update_preprocessor(&self) -> result::Result<(), JsValue> {
        self.object.emitPreprocessorChange()
    }

    #[profile(Debug)]
    #[cfg(target_arch = "wasm32")]
    /// Helper method to call methods on the wrapped javascript object.
    fn try_call1(
        &self,
        method: &Option<web::Function>,
        arg: &JsValue,
    ) -> result::Result<(), JsValue> {
        if let Some(method) = method {
            if let Err(error) = method.call1(&self.object, arg) {
                warn!("Failed to call method {method:?} with error: {error:?}");
                return Err(error);
            }
        }
        Ok(())
    }

    fn set_layer(&self, layer: Layer) {
        layer.apply_for_html_component(&self.scene, &self.root_node)
    }

    fn set_active(&self, active: bool) {
        let attribute = if active { "all" } else { "none " };
        self.root_node.set_style_or_warn("pointer-events", attribute);
    }
}



// ================
// === Instance ===
// ================

/// Sample visualization that renders the given data as text. Useful for debugging and testing.
#[derive(Clone, CloneRef, Debug, Deref)]
#[allow(missing_docs)]
pub struct Instance {
    #[deref]
    model:   InstanceModel,
    frp:     visualization::instance::Frp,
    network: frp::Network,
}

impl Instance {
    /// Constructor.
    pub fn new(class: &JsValue, app: &Application) -> result::Result<Instance, Error> {
        let scene = &app.display.default_scene;
        let network = frp::Network::new("js_visualization_instance");
        let frp = visualization::instance::Frp::new(&network);
        let model = InstanceModel::from_class(class, scene)?;
        model.set_dom_layer(&scene.dom.layers.back);
        model.set_active(false);
        Ok(Instance { model, frp, network }.init_frp().init_preprocessor_change_callback())
    }

    fn init_frp(self) -> Self {
        let network = &self.network;
        let model = self.model.clone_ref();
        let frp = self.frp.clone_ref();
        frp::extend! { network
            eval frp.set_size  ((size) model.set_size(*size));
            eval frp.send_data ([frp,model](data) {
                if let Err(e) = model.receive_data(data) {
                    frp.data_receive_error.emit(Some(e));
                }
            });
            eval frp.set_layer ((layer) model.set_layer(*layer));
            eval frp.is_active ((is_active) model.set_active(*is_active));
        }
        self
    }

    fn init_preprocessor_change_callback(self) -> Self {
        // FIXME Does it leak memory? To be checked.
        let change = self.frp.preprocessor_change.clone_ref();
        let callback = move |preprocessor_config| change.emit(preprocessor_config);
        let callback = Box::new(callback);
        self.model.preprocessor_change.borrow_mut().replace(callback);
        if let Err(err) = self.model.update_preprocessor() {
            error!(
                "Failed to trigger initial preprocessor update from JS: {}",
                err.print_to_string()
            );
        }
        self
    }
}

impl From<Instance> for visualization::Instance {
    fn from(t: Instance) -> Self {
        Self::new(&t, &t.frp, &t.network, Some(t.model.root_node.clone_ref()))
    }
}

impl display::Object for Instance {
    fn display_object(&self) -> &display::object::Instance {
        self.model.root_node.display_object()
    }
}


// === Utils ===

#[cfg(target_arch = "wasm32")]
/// Try to return the method specified by the given name on the given object as a
/// `web::Function`.
fn get_method(object: &web::Object, property: &str) -> Result<web::Function> {
    let method_value = web::Reflect::get(object, &property.into());
    let method_value = method_value.map_err(|object| Error::PropertyNotFoundOnObject {
        object,
        property: property.to_string(),
    })?;
    if method_value.is_undefined() {
        let object: JsValue = object.into();
        return Err(Error::PropertyNotFoundOnObject { object, property: property.to_string() });
    }
    let method_function: web::Function = method_value.into();
    Ok(method_function)
}

#[cfg(not(target_arch = "wasm32"))]
fn get_method(
    _object: &java_script::binding::Visualization,
    _property: &str,
) -> Result<web::Function> {
    Ok(default())
}
