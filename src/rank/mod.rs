pub mod criterion;
mod query_builder;
mod distinct_map;

use std::iter::FusedIterator;
use std::slice::Iter;
use std::ops::Range;

use sdset::SetBuf;
use group_by::GroupBy;

use crate::{Match, DocumentId};

pub use self::query_builder::{FilterFunc, QueryBuilder, DistinctQueryBuilder};

#[inline]
fn match_query_index(a: &Match, b: &Match) -> bool {
    a.query_index == b.query_index
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: DocumentId,
    pub matches: Matches,
}

impl Document {
    pub fn new(id: DocumentId, match_: Match) -> Self {
        let matches = SetBuf::new_unchecked(vec![match_]);
        Self::from_matches(id, matches)
    }

    pub fn from_matches(id: DocumentId, matches: SetBuf<Match>) -> Self {
        let matches = Matches::new(matches);
        Self { id, matches }
    }

    pub fn from_unsorted_matches(id: DocumentId, matches: Vec<Match>) -> Self {
        let matches = Matches::from_unsorted(matches);
        Self { id, matches }
    }
}

#[derive(Debug, Clone)]
pub struct Matches {
    matches: SetBuf<Match>,
    slices: Vec<Range<usize>>,
}

impl Matches {
    pub fn new(matches: SetBuf<Match>) -> Matches {
        let mut last_end = 0;
        let mut slices = Vec::new();

        for group in GroupBy::new(&matches, match_query_index) {
            let start = last_end;
            let end = last_end + group.len();
            slices.push(Range { start, end });
            last_end = end;
        }

        Matches { matches, slices }
    }

    pub fn from_unsorted(mut matches: Vec<Match>) -> Matches {
        matches.sort_unstable();
        let matches = SetBuf::new_unchecked(matches);
        Matches::new(matches)
    }

    pub fn query_index_groups(&self) -> QueryIndexGroups {
        QueryIndexGroups {
            matches: &self.matches,
            slices: self.slices.iter(),
        }
    }

    pub fn as_matches(&self) -> &[Match] {
        &self.matches
    }
}

pub struct QueryIndexGroups<'a, 'b> {
    matches: &'a [Match],
    slices: Iter<'b, Range<usize>>,
}

impl<'a> Iterator for QueryIndexGroups<'a, '_> {
    type Item = &'a [Match];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.slices.next().cloned().map(|range| {
            unsafe { self.matches.get_unchecked(range) }
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.slices.size_hint()
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.slices.nth(n).cloned().map(|range| {
            unsafe { self.matches.get_unchecked(range) }
        })
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        let (matches, slices) = (self.matches, self.slices);
        slices.last().cloned().map(|range| {
            unsafe { matches.get_unchecked(range) }
        })
    }
}

impl ExactSizeIterator for QueryIndexGroups<'_, '_> {
    #[inline]
    fn len(&self) -> usize {
        self.slices.len()
    }
}

impl FusedIterator for QueryIndexGroups<'_, '_> { }

impl DoubleEndedIterator for QueryIndexGroups<'_, '_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.slices.next_back().cloned().map(|range| {
            unsafe { self.matches.get_unchecked(range) }
        })
    }
}
