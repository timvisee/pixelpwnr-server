use draw_state::state::RefValues;
use gfx::{
    handle::Manager,
    pso::{AccessInfo, DataBind, DataLink, PipelineData, RawDataSet},
    Resources,
};

const REF_VALUES: RefValues = RefValues {
    stencil: (0, 0),
    blend: [0.2, 0.2, 0.2, 0.2],
};

#[derive(Clone, Debug, PartialEq)]
pub struct RefValuesWrapper(RefValues);

impl<R: Resources> PipelineData<R> for RefValuesWrapper {
    type Meta = crate::stats_renderer::bg_pipe::Meta;

    fn bake_to(
        &self,
        _: &mut RawDataSet<R>,
        _: &Self::Meta,
        _: &mut Manager<R>,
        _: &mut AccessInfo<R>,
    ) {
    }
}

impl<R: Resources> DataBind<R> for RefValuesWrapper {
    type Data = ();

    fn bind_to(&self, out: &mut RawDataSet<R>, _: &(), _: &mut Manager<R>, _: &mut AccessInfo<R>) {
        out.ref_values = REF_VALUES;
    }
}

impl DataLink<'_> for RefValuesWrapper {
    type Init = RefValuesWrapper;

    fn new() -> Self {
        RefValuesWrapper(REF_VALUES)
    }

    fn is_active(&self) -> bool {
        true
    }
}

impl std::hash::Hash for RefValuesWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0
            .blend
            .iter()
            .for_each(|val| val.to_be_bytes().hash(state));
        self.0.stencil.hash(state);
    }
}
