use crate::MimeType;
use crate::MimeType::*;
use std::array::TryFromSliceError;
use std::ops::Range;
use uintx::{u120, u24, u40, u48, u96};

static SHEBANGS: [(&[&str], &[MimeType]); 3] = [
  (&["#!/bin/bash", "#!/usr/bin/bash", "#!/usr/bin/env bash"], &[ApplicationBourneShell]),
  (
    &["#!/usr/bin/python", "#!/usr/bin/python3", "#!/usr/bin/env python", "#!/usr/bin/env python3"],
    &[TextPython],
  ),
  (&["#!/usr/bin/lua", "#!/usr/bin/env lua"], &[TextLua]),
];

fn check_shebang(data: &[u8], prefix: &str) -> bool {
  if !data.starts_with(prefix.as_bytes()) {
    return false;
  }

  let suffix = data.get(prefix.len());

  if suffix == Some(&b'\n') {
    return true;
  }

  if suffix == Some(&b'\r') && data.get(prefix.len() + 1) == Some(&b'\n') {
    return true;
  }

  false
}

fn check_shebangs(data: &[u8]) -> Option<&'static [MimeType]> {
  if !data.starts_with(b"#!") {
    return None;
  }

  for (start, types) in SHEBANGS {
    for ele in start {
      if check_shebang(data, ele) {
        return Some(types);
      }
    }
  }

  None
}

fn slice_it<'a, T: TryFrom<&'a [u8], Error = TryFromSliceError>, X>(
  data: &'a [u8],
  position: Range<usize>,
  mapper: impl Fn(T) -> X,
) -> Option<X> {
  data.get(position).map(T::try_from).and_then(Result::ok).map(mapper)
}

#[allow(clippy::useless_asref, clippy::collapsible_match, clippy::single_match)] //Readability of this fn is more important than making clippy happy.
fn handle_utf8(data: &[u8]) -> &'static [MimeType] {
  if let Some(bang) = check_shebangs(data) {
    return bang;
  }

  if let Some(num) = slice_it(data, 0..15, u120::from_be_bytes) {
    match num.as_num() {
      0x3C21444F43545950452068746D6C3E => return &[TextHtml],
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..6, u48::from_be_bytes) {
    match num.as_num() {
      0x3C68746D6C3E => return &[TextHtml],
      0x7B5C72746631 => return &[ApplicationRichText],
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..5, u40::from_be_bytes) {
    match num.as_num() {
      0x3C3F786D6C => return &[ApplicationXml],
      _ => (),
    }
  }

  //TODO this is probably utf8 text?
  &[TextPlain]
}

#[allow(clippy::useless_asref, clippy::collapsible_match, clippy::single_match)] //Readability of this fn is more important than making clippy happy.
pub(crate) fn typeify_header(data: &[u8]) -> &'static [MimeType] {
  let data = data.as_ref();

  if let Some(num) = slice_it(data, 0..15, u120::from_be_bytes) {
    match num.as_num() {
      0x3C21444F43545950452068746D6C3E => return &[TextHtml],
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..12, u96::from_be_bytes) {
    match num.as_num() {
      0xFFD8FFE000104A4649460001 => return &[ImageJpeg],
      0x000100000013010000040030 => return &[FontTtf],
      _ => (),
    }

    match num.as_num() & 0xFFFFFF_0000FFFF_FFFFFFFF {
      0xFFD8FFE1_00004578_69660000 => return &[ImageJpeg],
      _ => (),
    }

    match num.as_num() & 0xFFFFFFFF_00000000_FFFFFFFF {
      0x52494646_00000000_57415645 => return &[AudioWaveform],
      0x52494646_00000000_41564920 => return &[VideoAvi],
      0x52494646_00000000_57454250 => return &[ImageWebp],
      _ => (),
    }

    match num.as_num() & 0x00000000FFFFFFFFFFFFFFFF {
      0x000000006674797069736F6D | 0x00000000667479704D534E56 => return &[VideoMp4, AudioMp4],
      0x000000006674797061766966 => return &[ImageAvif],
      0x000000006674797068656963 => return &[ImageHeic],
      0x00000000667479704d344120 => return &[AudioMp4],
      0x000000006674797033677034 => return &[Video3gpp, Video3gpp2, Audio3gpp, Audio3gpp2],
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..8, u64::from_be_bytes) {
    match num {
      0x89504E470D0A1A0A => return &[ImagePng],
      0xD0CF11E0A1B11AE1 => {
        return &[
          ApplicationMicrosoftWord,
          ApplicationMicrosoftExcel,
          ApplicationMicrosoftPowerpoint,
          ApplicationMicrosoftVisio,
          ApplicationMicrosoftInstaller,
        ]
      }
      _ => (),
    };
  }

  if let Some(num) = slice_it(data, 0..6, u48::from_be_bytes) {
    match num.as_num() {
      0xFD377A585A00 => return &[ApplicationXz],
      0x377ABCAF271C => return &[Application7Zip],
      0x3C68746D6C3E => return &[TextHtml],
      0x7B5C72746631 => return &[ApplicationRichText],
      0x474946383761 | 0x474946383961 => return &[ImageGif],
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..5, u40::from_be_bytes) {
    match num.as_num() {
      0x3C3F786D6C => return &[ApplicationXml],
      0x255044462D => return &[ApplicationPdf],
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..4, u32::from_be_bytes) {
    match num {
      0x7F454C46 => return &[ApplicationElf],
      0xCAFEBABE => return &[ApplicationJavaClass],
      0x52617221 => return &[ApplicationRar],
      0x504B0304 | 0x504B0506 | 0x504B0708 => {
        return &[ApplicationZip, ApplicationJar, ApplicationEpub]
      }
      0xFFD8FFDB | 0xFFD8FFEE | 0xFFD8FFE0 => return &[ImageJpeg],
      0x49492A00 | 0x4D4D002A | 0x49492B00 | 0x4D4D002B => return &[ImageTiff],
      0x0061736D => return &[ApplicationWasm],
      0x000001BA => return &[VideoMpeg],
      0x000001B3 => return &[VideoMpeg],
      0x00000100 => return &[ImageIcon],
      0x1B4C7561 => return &[ApplicationLuaBytecode],
      0x1A45DFA3 => return &[VideoWebm, AudioWebm],
      0x4F676753 => return &[VideoOgg, AudioOgg],
      0x4D546864 => return &[AudioMidi],
      0x716f6966 => return &[ImageQoi],
      _ => (),
    };
  }

  if let Some(num) = slice_it(data, 0..3, u24::from_be_bytes) {
    match num.as_num() {
      0xEFBBBF => return data.get(3..).map(handle_utf8).unwrap_or(&[]),
      _ => (),
    }
  }

  if let Some(num) = slice_it(data, 0..2, u16::from_be_bytes) {
    match num {
      0x1F8B => return &[ApplicationGzip],
      0x4D5A => return &[ApplicationDosMZExe],
      0xFFFB => return &[AudioMp3],
      0xFFFD => return &[AudioMpeg],
      0xFFF1 => return &[AudioAac],
      0x424D => return &[ImageBmp],
      _ => (),
    }
  }

  if let Some(bang) = check_shebangs(data) {
    return bang;
  }

  if data.starts_with(&[0u8; 128])
    && slice_it(data, 128..132, u32::from_be_bytes) == Some(0x4449434D)
  {
    return &[ApplicationDicom];
  }

  if let Some(num) = slice_it(data, 257..265, u64::from_be_bytes) {
    match num {
      0x7573746172003030 | 0x7573746172202000 => return &[ApplicationTapeArchive],
      _ => (),
    }
  }

  if data.get(4).copied() == Some(0x47)
    && data.get(196).copied() == Some(0x47)
    && data.get(388).copied() == Some(0x47)
  {
    return &[VideoMpegTransportStream];
  }

  &[]
}

#[test]
fn test_xz() {
  let data = vec![0xFDu8, 0x37, 0x7A, 0x58, 0x5A, 0x00];
  let xx = typeify_header(&data);
  assert_eq!(xx, &[ApplicationXz])
}

#[test]
fn test_utf8_html() {
  let xx = typeify_header("<!DOCTYPE html>".as_bytes());
  assert_eq!(xx, &[TextHtml]);

  let xx = typeify_header("\u{FEFF}<!DOCTYPE html>".as_bytes());
  assert_eq!(xx, &[TextHtml]);
}

#[test]
fn test_text() {
  let xx = typeify_header("\u{FEFF}Hello World".as_bytes());
  assert_eq!(xx, &[TextPlain]);
}
