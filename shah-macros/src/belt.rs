use crate::utils::ez_trait::ez_trait;

ez_trait! {
    belt,
    belt::Belt,
    "next", models::Gene, next_ident, next, next_mut;
    "past", models::Gene, past_ident, past, past_mut;
    "buckle", models::Gene, buckle_ident, buckle, buckle_mut;
}

ez_trait! {
    buckle,
    belt::Buckle,
    "head", models::Gene, head_ident, head, head_mut;
    "tail", models::Gene, tail_ident, tail, tail_mut;
    "belt_count", u64, belt_count_ident, belt_count, belt_count_mut;
}
