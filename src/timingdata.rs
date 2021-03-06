use crate::notedata::{ChartData, ChartMetadata, NoteData, NoteType};
use ggez::graphics;
use num_rational::Rational32;
use std::slice;

fn value(fraction: Rational32) -> f64 {
    *fraction.numer() as f64 / *fraction.denom() as f64
}

#[derive(Debug, PartialEq)]
pub struct TimingData<T>
where
    T: TimingInfo,
{
    notes: [Vec<T>; 4],
}

pub trait TimingInfo {}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GameplayInfo(pub i64, pub graphics::Rect, pub NoteType);

impl TimingInfo for GameplayInfo {}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OffsetInfo(pub Option<i64>, pub NoteType);

impl TimingInfo for OffsetInfo {}

impl OffsetInfo {
    fn wife(self, ts: f64) -> f64 {
        match self.1 {
            NoteType::Tap | NoteType::Hold | NoteType::Roll | NoteType::Lift => {
                let maxms = match self.0 {
                    Some(offset) => offset,
                    None => return -8.0,
                } as f64;
                let avedeviation = 95.0 * ts;
                let mut y =
                    1.0 - 2.0_f64.powf(-1.0 * maxms * maxms / (avedeviation * avedeviation));
                y *= y;
                (10.0) * (1.0 - y) - 8.0
            }
            NoteType::Fake | NoteType::HoldEnd => 0.0,
            NoteType::Mine => match self.0 {
                Some(_) => -8.0,
                None => 0.0,
            },
        }
    }
    fn max_points(self) -> f64 {
        match self.1 {
            NoteType::Tap | NoteType::Hold | NoteType::Roll | NoteType::Lift => 2.0,
            NoteType::Fake | NoteType::Mine | NoteType::HoldEnd => 0.0,
        }
    }
}

impl TimingData<GameplayInfo> {
    pub fn from_notedata<U>(data: &NoteData, sprite_finder: U, rate: f64) -> Vec<Self>
    where
        U: Fn(usize, f64, Rational32, NoteType, usize) -> graphics::Rect,
    {
        let metadata = &data.data;
        data.charts()
            .map(|chart| TimingData::from_chartdata::<U>(chart, metadata, &sprite_finder, rate))
            .collect()
    }
    pub fn from_chartdata<U>(
        data: &ChartData,
        meta: &ChartMetadata,
        sprite_finder: &U,
        rate: f64,
    ) -> Self
    where
        U: Fn(usize, f64, Rational32, NoteType, usize) -> graphics::Rect,
    {
        let offset = meta.offset.unwrap_or(0.0) * 1000.0;
        let mut bpms: Vec<_> = meta
            .bpms
            .iter()
            .map(|(x, y, z)| (*x, *y, *z, 0.0))
            .collect();
        match bpms.get_mut(0) {
            Some(bpm) => bpm.3 = offset,
            None => return TimingData::new(),
        };
        for i in 1..bpms.len() {
            bpms[i].3 = bpms[i - 1].3
                + (((bpms[i].0 - bpms[i - 1].0) as f64 + value(bpms[i].1 - bpms[i - 1].1))
                    * 240_000.0
                    / bpms[i - 1].2);
        }
        let mut bpms = bpms.into_iter();
        let mut current_bpm = bpms.next().unwrap();
        let mut next_bpm = bpms.next();
        let mut output = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (measure_index, measure) in data.columns().enumerate() {
            for (inner_time, row) in measure.iter() {
                if let Some(bpm) = next_bpm {
                    if measure_index as i32 > bpm.0
                        || (measure_index as i32 == bpm.0 && bpm.1 <= inner_time.fract())
                    {
                        current_bpm = bpm;
                        next_bpm = bpms.next();
                    }
                }
                let row_time = (current_bpm.3
                    + 240_000.0
                        * ((measure_index - current_bpm.0 as usize) as f64
                            + value(inner_time - current_bpm.1))
                        / current_bpm.2)
                    / rate;
                for (note, column_index) in row.notes() {
                    let sprite =
                        sprite_finder(measure_index, 0.0, *inner_time, *note, *column_index);
                    //This if let can hide errors in the parser or .sm file
                    // An else clause should be added where errors are handled
                    if let Some(column) = output.get_mut(*column_index) {
                        column.push(GameplayInfo(row_time as i64, sprite, *note));
                    }
                }
            }
        }
        TimingData { notes: output }
    }
}

impl<T> TimingData<T>
where
    T: TimingInfo,
{
    pub fn add(&mut self, offset: T, column: usize) {
        self.notes[column].push(offset);
    }
    pub fn columns(&self) -> slice::Iter<Vec<T>> {
        self.notes.iter()
    }
    pub fn new() -> Self {
        TimingData {
            notes: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
    }
}
impl TimingData<OffsetInfo> {
    pub fn calculate_score(&self) -> f64 {
        let max_points = self
            .columns()
            .flat_map(|x| x.iter())
            .map(|x| x.max_points())
            .sum::<f64>();
        let current_points = self
            .columns()
            .flat_map(|x| x.iter())
            .map(|x| x.wife(1.0))
            .sum::<f64>();
        current_points / max_points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wife_symmetry() {
        for offset in 0..180 {
            let early = OffsetInfo(Some(-offset), NoteType::Tap);
            let late = OffsetInfo(Some(offset), NoteType::Tap);
            assert_eq!(early.wife(1.0), late.wife(1.0));
        }
    }
    #[test]
    fn wife_peak() {
        assert_eq!(OffsetInfo(Some(0), NoteType::Tap).wife(1.0), 2.0);
        assert_eq!(OffsetInfo(Some(0), NoteType::Tap).wife(0.5), 2.0);
        assert_eq!(OffsetInfo(Some(0), NoteType::Tap).wife(2.0), 2.0);
    }
    #[test]
    fn wife_decreasing() {
        for offset in 0..179 {
            assert!(
                OffsetInfo(Some(offset), NoteType::Tap).wife(1.0)
                    > OffsetInfo(Some(offset + 1), NoteType::Tap).wife(1.0)
            );
            assert!(
                OffsetInfo(Some(offset), NoteType::Tap).wife(0.5)
                    > OffsetInfo(Some(offset + 1), NoteType::Tap).wife(0.5)
            );
            assert!(
                OffsetInfo(Some(offset), NoteType::Tap).wife(2.0)
                    > OffsetInfo(Some(offset + 1), NoteType::Tap).wife(2.0)
            );
        }
    }
}
