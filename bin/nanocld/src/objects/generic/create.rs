use nanocl_error::io::IoResult;
use nanocl_stubs::system::{EventActor, NativeEventAction};

use crate::utils;

use crate::models::SystemState;

/// A Create trait for all objects in Nanocl
/// It will automatically emit events
/// when an object is created,  etc.deleted, updated
/// You need to implement the `fn_create_obj` function
/// That will perform the create action and return the object
/// Then you can use the `create_obj` function
pub trait ObjCreate {
  type ObjCreateIn;
  type ObjCreateOut;

  async fn fn_create_obj(
    obj: &Self::ObjCreateIn,
    state: &SystemState,
  ) -> IoResult<Self::ObjCreateOut>;

  async fn create_obj(
    obj: &Self::ObjCreateIn,
    state: &SystemState,
  ) -> IoResult<Self::ObjCreateOut>
  where
    Self::ObjCreateOut: Into<EventActor> + Clone,
  {
    let obj = Self::fn_create_obj(obj, state).await?;
    utils::event_emitter::emit_normal_native_action(
      &obj,
      NativeEventAction::Create,
      state,
    );
    Ok(obj)
  }
}
