pub mod frame_anim;
pub mod move_seq;
pub mod stand;

use slotmap::SecondaryMap;
use std::time::Instant;

use crate::game::object::Handle;
use crate::sequence::chain::*;
use crate::sequence::*;
use std::rc::Rc;
use std::cell::RefCell;

struct Seq {
    main: Control,
    subs: Vec<Control>,
}

impl Seq {
    fn cancel(&mut self) {
        for sub in self.subs.drain(..) {
            sub.cancel();
        }
    }
}

/// Tracks sequences bound to objects.
pub struct ObjSequencer {
    seqr: Sequencer,
    seqs: SecondaryMap<Handle, Seq>,
    done_objs: Rc<RefCell<Vec<Handle>>>,
}

impl ObjSequencer {
    pub fn new(now: Instant) -> Self {
        Self {
            seqr: Sequencer::new(now),
            seqs: Default::default(),
            done_objs: Default::default(),
        }
    }

    /// Clears all sequences including the finalizing sequences.
    pub fn clear(&mut self) {
        self.seqr.stop_all();
        self.seqs.clear();
        self.done_objs.borrow_mut().clear();
    }

    pub fn is_running(&self, object: Handle) -> bool {
        self.seqs.contains_key(object)
    }

    /// Cancels all sequences running for `object`.
    pub fn cancel(&mut self, object: Handle) {
        if let Some(mut seq) = self.seqs.remove(object) {
            seq.cancel();
        }
    }

    /// Cancels all sequences running for `object` and starts a new `chain` sequence.
    pub fn replace(&mut self, object: Handle, chain: Chain) {
        if !self.seqs.contains_key(object) {
            let main = Chain::new();

            let done_objs = self.done_objs.clone();
            main.control().on_done(move || done_objs.borrow_mut().push(object));

            self.seqs.insert(object, Seq {
                main: main.control().clone(),
                subs: Vec::new(),
            });
            self.seqr.start(main);
        }
        let seq = self.seqs.get_mut(object).unwrap();
        seq.cancel();

        seq.subs.push(chain.control().clone());
        seq.main.cancellable(chain);
    }

    pub fn update(&mut self, ctx: &mut Update) {
        self.seqr.update(ctx);
        self.remove_done_objs();
    }

    pub fn sync(&mut self, ctx: &mut Sync) {
        self.seqr.sync(ctx);
        self.remove_done_objs();
    }

    fn remove_done_objs(&mut self) {
        for obj in self.done_objs.borrow_mut().drain(..) {
            self.seqs.remove(obj);
        }
    }
}