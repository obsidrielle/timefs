use std::collections::HashMap;
use fuser::FileAttr;
use crate::block::BlockRef;

pub(crate) enum INodeType {
    File {
        blocks: Vec<BlockRef>,
        size: u64,
    },
    Directory {
        entries: HashMap<String, u64>,
    }
}

pub(crate) struct INode {
    id: u64,
    parent: u64,
    data: INodeType,
    attr: FileAttr,
}
