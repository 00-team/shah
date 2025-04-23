use crate::utils::ez_trait::ez_trait;

ez_trait! {
    pond,
    pond::Pond,
    "next", models::Gene, next_ident, next, next_mut;
    "past", models::Gene, past_ident, past, past_mut;
    "origin", models::Gene, origin_ident, origin, origin_mut;
    "stack", models::GeneId, stack_ident, stack, stack_mut;
    "alive",  exp::u8, alive_ident, alive, alive_mut;
    "empty", exp::u8, empty_ident, empty, empty_mut;
}

ez_trait! {
    duck,
    pond::Duck,
    "pond", models::Gene, pond_ident, pond, pond_mut;
}

ez_trait! {
    origin,
    pond::Origin,
    "head", models::Gene, head_ident, head, head_mut;
    "tail", models::Gene, tail_ident, tail, tail_mut;
    "pond_count", exp::u64, pond_count_ident, pond_count, pond_count_mut;
    "item_count", exp::u64, item_count_ident, item_count, item_count_mut;
}
