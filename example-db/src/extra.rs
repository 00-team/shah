use crate::models::{ExampleError, State};
use shah::db::belt::Buckle;
use shah::models::{Binary, Gene};
use shah::{db::belt::BeltDb, ShahError};
use shah::{AsUtf8Str, ClientError, ErrorCode, OptNotFound, Taker, BLOCK_SIZE};

#[allow(unused_imports)]
pub use client::*;
pub use db::Extra;

const EXTRA_DATA: usize = BLOCK_SIZE * 2 - 3;

pub(crate) mod db {
    use super::*;

    #[derive(shah::ShahSchema)]
    #[shah::model]
    #[derive(Debug, shah::Entity, shah::Belt)]
    pub struct Extra {
        pub gene: Gene,
        pub next: Gene,
        pub past: Gene,
        pub buckle: Gene,
        growth: u64,
        pub length: u16,
        entity_flags: u8,
        pub data: [u8; EXTRA_DATA],
    }

    pub type ExtraDb = BeltDb<Extra>;

    #[allow(dead_code)]
    pub fn init() -> Result<ExtraDb, ShahError> {
        ExtraDb::new("extra", 1)
    }
}

#[shah::api(scope = 3, error = crate::models::ExampleError)]
mod eapi {
    use super::*;

    pub(crate) fn buckle_get(
        state: &mut State, (buckle_gene,): (&Gene,), (buckle,): (&mut Buckle,),
    ) -> Result<(), ErrorCode> {
        state.extra.buckle_get(buckle_gene, buckle)?;
        Ok(())
    }

    pub(crate) fn buckle_add(
        state: &mut State, _: (), (out,): (&mut Buckle,),
    ) -> Result<(), ErrorCode> {
        out.zeroed();
        state.extra.buckle_add(out)?;
        Ok(())
    }

    pub(crate) fn buckle_del(
        state: &mut State, (buckle_gene,): (&Gene,), _: (),
    ) -> Result<(), ErrorCode> {
        state.extra.buckle_del(buckle_gene)?;
        Ok(())
    }

    pub(crate) fn add(
        state: &mut State, (buckle_gene, extra): (&Gene, &Extra),
        (out,): (&mut Extra,),
    ) -> Result<(), ErrorCode> {
        out.clone_from(extra);
        state.extra.belt_add(buckle_gene, out)?;
        Ok(())
    }

    pub(crate) fn get(
        state: &mut State, (extra_gene,): (&Gene,), (belt,): (&mut Extra,),
    ) -> Result<(), ErrorCode> {
        state.extra.belt_get(extra_gene, belt)?;
        Ok(())
    }

    pub(crate) fn set(
        state: &mut State, (extra,): (&Extra,), (res,): (&mut Extra,),
    ) -> Result<(), ErrorCode> {
        res.clone_from(extra);
        state.extra.belt_set(res)?;
        Ok(())
    }

    pub(crate) fn del(
        state: &mut State, (extra_gene,): (&Gene,), (res,): (&mut Extra,),
    ) -> Result<(), ErrorCode> {
        state.extra.belt_del(extra_gene, res)?;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn get_all(
    taker: &Taker, buckle_gene: &Gene,
) -> Result<String, ClientError<ExampleError>> {
    let buckle = buckle_get(taker, buckle_gene)?;
    let mut data = Vec::with_capacity(buckle.belts as usize * EXTRA_DATA);

    let mut gene = buckle.head;
    loop {
        if gene.is_none() {
            break;
        }

        let Some(extra) = get(taker, &gene).onf()? else { break };
        let len = (extra.length as usize).min(extra.data.len());
        data.extend_from_slice(&extra.data[..len]);
        gene = extra.next;
    }

    Ok(data[..].as_utf8_str_null_terminated().to_string())
}

#[allow(dead_code)]
pub fn set_all(
    taker: &Taker, buckle_gene: &Option<Gene>, data: &str,
) -> Result<Gene, ClientError<ExampleError>> {
    let data = data.as_bytes();

    let b = if let Some(og) = buckle_gene {
        buckle_get(taker, og).onf()?
    } else {
        None
    };
    let buckle = if let Some(b) = b { b } else { buckle_add(taker)? };
    let mut gene = buckle.head;
    let mut extra = Extra::default();

    for x in data.chunks(EXTRA_DATA) {
        extra.data[..x.len()].copy_from_slice(x);
        extra.length = x.len() as u16;

        if gene.is_none() {
            extra.gene.clear();
            add(taker, &buckle.gene, &extra)?;
            continue;
        }

        extra.gene = gene;
        match set(taker, &extra).onf()? {
            Some(v) => gene = v.next,
            None => {
                extra.gene.clear();
                add(taker, &buckle.gene, &extra)?;
                gene.clear();
            }
        }
    }

    Ok(buckle.gene)
}
