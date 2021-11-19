use crate::newtypes::libcnb_newtype;

libcnb_newtype!(
    layer,
    layer_name,
    LayerName,
    LayerNameError,
    r"^(?!build|launch|store).*$"
);
