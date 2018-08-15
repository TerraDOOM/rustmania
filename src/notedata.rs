use std::fs;
use std::slice;
use fraction::Fraction;
use nom::double_s;

#[derive(Debug)]
pub struct ChartMetadata {
    pub title: Option<String>,
    pub offset: Option<f64>,
    pub bpm: Option<f64>,
}

#[derive(Debug)]
pub struct NoteData {
    notes: Vec<Vec<(Fraction, NoteRow)>>,
    pub data: ChartMetadata,
}

#[derive(Debug)]
pub struct NoteRow {
    pub row: Vec<(NoteType,usize)>,
}

#[derive(Debug)]
pub enum NoteType {
    Tap,
    Hold,
    Roll,
    Mine,
    Lift,
    Fake,
}

named!( bpm_parse<&str,(Vec<(f64,f64)>,(f64,f64))>, many_till!(bpm_line, do_parse!(
time: double_s >>
tag!("=")   >>
bpm: double_s >>
tag!(";")    >>
( ( time, bpm ) )
)));

named!(bpm_line<&str, (f64,f64)>,
  do_parse!(
        time: double_s >>
           tag!("=")   >>
           bpm: double_s >>
           tag!(",")    >>
    ( ( time, bpm ) )
  )
);

named!(float_tag_parse<&str, f64>,
    do_parse!(
        value: double_s >>
        tag!(";") >>
    ( value )
));

impl ChartMetadata {
    pub fn new() -> Self {
        ChartMetadata {
            title: None,
            offset: None,
            bpm: None,
        }
    }
}


fn parse_measure(measure: &[&str]) -> Vec<(Fraction,NoteRow)> {
    let mut output = Vec::new();
    let division = measure.len();
    for (subindex, beat) in measure.iter().enumerate() {
        output.push((Fraction::new(subindex as i64,division as u64).unwrap(),parse_line(beat)));
    }
    output
}

fn parse_line(contents: &&str) -> NoteRow {
    let mut row = Vec::new();
    contents.chars().enumerate().for_each(|(index, character)| {
        if let Some(note) = char_to_notetype(character) {
            row.push((note,index));
        }
    });
    NoteRow {
        row,
    }
}

fn char_to_notetype(character: char) -> Option<NoteType> {
    match character {
        '0' => None,
        '1' => Some(NoteType::Tap),
        '2' => Some(NoteType::Hold),
        '4' => Some(NoteType::Roll),
        'M' => Some(NoteType::Mine),
        'L' => Some(NoteType::Lift),
        'F' => Some(NoteType::Fake),
        _ => None
    }
}

fn parse_main_block(contents: String) -> Vec<Vec<(Fraction, NoteRow)>> {
    let mut notes = Vec::new();
    let lines: Vec<_> = contents.lines().skip(5).collect();
    let measures = lines.split(|&x| x == ",");
    for measure in measures {
        notes.push(parse_measure(measure));
    }
    notes
}

fn split_once(contents: &str, letter: char) -> (&str,&str) {
    let mut split = contents.splitn(2, letter);
    let first = split.next().unwrap_or("");
    let second = split.next().unwrap_or("");
    (first,second)
}

fn parse_tag(tag: &str, contents: &str, data: &mut NoteData) {
    match tag {
        "TITLE" => data.data.title = Some(contents.to_string()),
        "OFFSET" => data.data.offset = match float_tag_parse(contents) {
            Ok(thing) => Some(-1.0*thing.1),
            Err(_) => None,
        },
        "BPMS" => data.data.bpm = match bpm_parse(contents) {
            Ok(thing) => Some(((thing.1).1).1),
            Err(_) => None,
        },
        "NOTES" => data.notes = parse_main_block(contents.to_string()),
        _ => {},
    }
}

impl NoteData {
    pub fn from_sm() -> Self {
        let mut chart = NoteData {
                notes: Vec::new(),
                data: ChartMetadata::new(),
            };
        let simfile = fs::read_to_string("resources/barebones.sm").unwrap();
        let tags = simfile.split(|x| x == '#').map(|x| split_once(x, ':'));
        for (tag, contents) in tags {
            parse_tag(tag, contents, &mut chart);
        }
        chart
    }
    pub fn columns(&self) -> slice::Iter<Vec<(Fraction, NoteRow)>> {
        self.notes.iter()
    }
}