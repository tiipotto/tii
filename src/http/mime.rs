//! Provides functionality for handling MIME types.

use crate::util::unwrap_some;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};

/// QValue is defined as a fixed point number with up to 3 digits
/// after comma. with a valid range from 0 to 1.
/// We represent this as an u16 from 0 to 1000.
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(transparent)]
pub struct TiiQValue(u16);

impl TiiQValue {
  /// q=1.0
  pub const MAX: TiiQValue = TiiQValue(1000);

  /// q=0.0
  pub const MIN: TiiQValue = TiiQValue(0);

  /// Parses the QValue in http header representation.
  /// Note: this is without the "q=" prefix!
  /// Returns none if the value is either out of bounds or otherwise invalid.
  pub fn parse(qvalue: impl AsRef<str>) -> Option<TiiQValue> {
    let qvalue = qvalue.as_ref();
    match qvalue.len() {
      1 => {
        if qvalue == "1" {
          return Some(TiiQValue(1000));
        }
        if qvalue == "0" {
          return Some(TiiQValue(0));
        }

        None
      }
      2 => None,
      3 => {
        if !qvalue.starts_with("0.") {
          if qvalue == "1.0" {
            return Some(TiiQValue(1000));
          }
          return None;
        }

        if let Ok(value) = qvalue[2..].parse::<u16>() {
          return Some(TiiQValue(value * 100));
        }

        None
      }
      4 => {
        if !qvalue.starts_with("0.") {
          if qvalue == "1.00" {
            return Some(TiiQValue(1000));
          }
          return None;
        }

        if let Ok(value) = qvalue[2..].parse::<u16>() {
          return Some(TiiQValue(value * 10));
        }

        None
      }
      5 => {
        if !qvalue.starts_with("0.") {
          if qvalue == "1.000" {
            return Some(TiiQValue(1000));
          }
          return None;
        }

        if let Ok(value) = qvalue[2..].parse::<u16>() {
          return Some(TiiQValue(value));
        }

        None
      }
      _ => None,
    }
  }

  /// Returns the QValue in http header representation.
  /// Note: this is without the "q=" prefix!
  pub const fn as_str(&self) -> &'static str {
    tii_procmacro::qvalue_to_strs!()
  }

  /// returns this QValue as an u16. This value always ranges from 0 to 1000.
  /// 1000 corresponds to 1.0 since q-values are fixed point numbers with up to 3 digits after comma.
  pub const fn as_u16(&self) -> u16 {
    self.0
  }

  /// Returns a QValue from the given u16. Parameters greater than 1000 are clamped to 1000.
  pub const fn from_clamped(qvalue: u16) -> TiiQValue {
    if qvalue > 1000 {
      return TiiQValue(1000);
    }

    TiiQValue(qvalue)
  }
}

impl Display for TiiQValue {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}
impl Default for TiiQValue {
  fn default() -> Self {
    TiiQValue(1000)
  }
}

/// Version of MimeType that can contain "*" symbols.
#[derive(Clone, PartialEq, Debug, Eq, Hash)]
pub enum TiiAcceptMimeType {
  /// video/* or text/* or ...
  GroupWildcard(TiiMimeGroup),
  /// text/html or application/json or ...
  Specific(TiiMimeType),
  /// */*
  Wildcard,
}

impl AsRef<TiiAcceptMimeType> for TiiAcceptMimeType {
  fn as_ref(&self) -> &TiiAcceptMimeType {
    self
  }
}

impl TiiAcceptMimeType {
  /// Parses an accept mime type.
  pub fn parse(value: impl AsRef<str>) -> Option<TiiAcceptMimeType> {
    let mime = value.as_ref();
    let mime = mime.split_once(";").map(|(mime, _)| mime).unwrap_or(mime);

    if mime == "*/*" {
      return Some(TiiAcceptMimeType::Wildcard);
    }
    match TiiMimeType::parse(mime) {
      None => match TiiMimeGroup::parse(mime) {
        Some(group) => {
          if &mime[group.as_str().len()..] != "/*" {
            return None;
          }

          Some(TiiAcceptMimeType::GroupWildcard(group))
        }
        None => None,
      },
      Some(mime) => Some(TiiAcceptMimeType::Specific(mime)),
    }
  }

  /// Returns true if this AcceptMimeType permits the given mime type.
  pub fn permits_specific(&self, mime_type: impl AsRef<TiiMimeType>) -> bool {
    match self {
      TiiAcceptMimeType::GroupWildcard(group) => group == mime_type.as_ref().mime_group(),
      TiiAcceptMimeType::Specific(mime) => mime == mime_type.as_ref(),
      TiiAcceptMimeType::Wildcard => true,
    }
  }

  /// Returns true if this AcceptMimeType will accept ANY mime from the given group.
  pub fn permits_group(&self, mime_group: impl AsRef<TiiMimeGroup>) -> bool {
    match self {
      TiiAcceptMimeType::GroupWildcard(group) => group == mime_group.as_ref(),
      TiiAcceptMimeType::Specific(_) => false,
      TiiAcceptMimeType::Wildcard => true,
    }
  }

  /// Returns true if this AcceptMimeType will permit ANY mime type permitted by the other AcceptMimeType.
  pub fn permits(&self, mime_type: impl AsRef<TiiAcceptMimeType>) -> bool {
    match mime_type.as_ref() {
      TiiAcceptMimeType::GroupWildcard(group) => self.permits_group(group),
      TiiAcceptMimeType::Specific(mime) => self.permits_specific(mime),
      TiiAcceptMimeType::Wildcard => matches!(self, TiiAcceptMimeType::Wildcard),
    }
  }
}

impl Display for TiiAcceptMimeType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      TiiAcceptMimeType::GroupWildcard(group) => {
        f.write_str(group.as_str())?;
        f.write_str("/*")?;
      }
      TiiAcceptMimeType::Specific(mime) => {
        f.write_str(mime.as_str())?;
      }
      TiiAcceptMimeType::Wildcard => f.write_str("*/*")?,
    }

    Ok(())
  }
}

///
/// Represents one part of an accept mime
/// # See
/// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept>
#[derive(Clone, PartialEq, Debug, Eq)]
pub struct TiiAcceptQualityMimeType {
  value: TiiAcceptMimeType,
  q: TiiQValue,
}

impl PartialOrd<Self> for TiiAcceptQualityMimeType {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TiiAcceptQualityMimeType {
  fn cmp(&self, other: &Self) -> Ordering {
    other.q.cmp(&self.q)
  }
}

impl TiiAcceptQualityMimeType {
  /// This fn parses an Accept header value from a client http request.
  /// The returned Vec is sorted in descending order of quality value q.
  pub fn parse(value: impl AsRef<str>) -> Option<Vec<Self>> {
    let value = value.as_ref();
    let mut data = Vec::new();
    for mut mime in value.split(",") {
      mime = mime.trim();

      if let Some((mime, rawq)) = mime.split_once(";") {
        if !rawq.starts_with("q=") {
          // TODO we dont support level notation...
          return None;
        }

        let qvalue = TiiQValue::parse(&rawq[2..])?;

        if mime == "*/*" {
          data.push(TiiAcceptQualityMimeType { value: TiiAcceptMimeType::Wildcard, q: qvalue });
          continue;
        }

        match TiiMimeType::parse(mime) {
          None => match TiiMimeGroup::parse(mime) {
            Some(group) => {
              if &mime[group.as_str().len()..] != "/*" {
                return None;
              }
              data.push(TiiAcceptQualityMimeType {
                value: TiiAcceptMimeType::GroupWildcard(group),
                q: qvalue,
              })
            }
            None => return None,
          },
          Some(mime) => data
            .push(TiiAcceptQualityMimeType { value: TiiAcceptMimeType::Specific(mime), q: qvalue }),
        };

        continue;
      }

      if mime == "*/*" {
        data.push(TiiAcceptQualityMimeType {
          value: TiiAcceptMimeType::Wildcard,
          q: TiiQValue::default(),
        });
        continue;
      }

      match TiiMimeType::parse(mime) {
        None => match TiiMimeGroup::parse(mime) {
          Some(group) => {
            if &mime[group.as_str().len()..] != "/*" {
              return None;
            }
            data.push(TiiAcceptQualityMimeType {
              value: TiiAcceptMimeType::GroupWildcard(group),
              q: TiiQValue::default(),
            })
          }
          None => return None,
        },
        Some(mime) => data.push(TiiAcceptQualityMimeType {
          value: TiiAcceptMimeType::Specific(mime),
          q: TiiQValue::default(),
        }),
      };
    }

    data.sort();
    Some(data)
  }

  /// Serializes a Vec of AcceptMime's into a full http header string.
  /// The returned string is guaranteed to work with the `parse` fn.
  pub fn elements_to_header_value(elements: &Vec<Self>) -> String {
    let mut buffer = String::new();
    for element in elements {
      if !buffer.is_empty() {
        buffer += ",";
      }
      buffer += element.to_string().as_str();
    }

    buffer
  }

  /// Gets the accept mime type without Q Value.
  pub fn get_type(&self) -> &TiiAcceptMimeType {
    &self.value
  }

  /// Get the QValue of this accept mime.
  pub const fn qvalue(&self) -> TiiQValue {
    self.q
  }

  /// Is this a */* accept?
  pub const fn is_wildcard(&self) -> bool {
    matches!(self.value, TiiAcceptMimeType::Wildcard)
  }

  /// Is this a group wildcard? i.e: `video/*` or `text/*`
  pub const fn is_group_wildcard(&self) -> bool {
    matches!(self.value, TiiAcceptMimeType::GroupWildcard(_))
  }

  /// Is this a non wildcard mime? i.e: `video/mp4`
  pub const fn is_specific(&self) -> bool {
    matches!(self.value, TiiAcceptMimeType::Specific(_))
  }

  /// Get the mime type. returns none if this is any type of wildcard mime
  pub const fn mime(&self) -> Option<&TiiMimeType> {
    match &self.value {
      TiiAcceptMimeType::Specific(mime) => Some(mime),
      _ => None,
    }
  }

  /// Get the mime type. returns none if this is the `*/*` mime.
  pub const fn group(&self) -> Option<&TiiMimeGroup> {
    match &self.value {
      TiiAcceptMimeType::Specific(mime) => Some(mime.mime_group()),
      TiiAcceptMimeType::GroupWildcard(group) => Some(group),
      _ => None,
    }
  }

  /// Returns a AcceptMime equivalent to calling parse with `*/*`
  pub const fn wildcard(q: TiiQValue) -> TiiAcceptQualityMimeType {
    TiiAcceptQualityMimeType { value: TiiAcceptMimeType::Wildcard, q }
  }

  /// Returns a AcceptMime equivalent to calling parse with `group/*` depending on MimeGroup.
  pub const fn from_group(group: TiiMimeGroup, q: TiiQValue) -> TiiAcceptQualityMimeType {
    TiiAcceptQualityMimeType { value: TiiAcceptMimeType::GroupWildcard(group), q }
  }

  /// Returns a AcceptMime equivalent to calling parse with `group/type` depending on MimeType.
  pub const fn from_mime(mime: TiiMimeType, q: TiiQValue) -> TiiAcceptQualityMimeType {
    TiiAcceptQualityMimeType { value: TiiAcceptMimeType::Specific(mime), q }
  }
}

impl Default for TiiAcceptQualityMimeType {
  fn default() -> Self {
    TiiAcceptQualityMimeType::wildcard(TiiQValue::default())
  }
}

impl Display for TiiAcceptQualityMimeType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.value, f)?;
    if self.q.as_u16() != 1000 {
      f.write_str(";q=")?;
      f.write_str(self.q.as_str())?;
    }
    Ok(())
  }
}

/// Mime types are split into groups denoted by whatever is before of the "/"
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum TiiMimeGroup {
  /// Fonts
  Font,
  /// Custom application specific things.
  Application,
  /// Images, anything that can be rendered onto a screen.
  Image,
  /// Video maybe with audio maybe without.
  Video,
  /// Audio
  Audio,
  /// Any human or pseudo human-readable text.
  Text,
  /// Anything else.
  Other(String),
}

impl AsRef<TiiMimeGroup> for TiiMimeGroup {
  fn as_ref(&self) -> &TiiMimeGroup {
    self
  }
}

impl Display for TiiMimeGroup {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

const WELL_KNOWN_GROUPS: &[TiiMimeGroup] = &[
  TiiMimeGroup::Font,
  TiiMimeGroup::Application,
  TiiMimeGroup::Image,
  TiiMimeGroup::Video,
  TiiMimeGroup::Audio,
  TiiMimeGroup::Text,
];
impl TiiMimeGroup {
  /// Parses a mime group from a str.
  /// This str can be either the mime group directly such as "video"
  /// or the full mime type such as "video/mp4"
  /// or the accept mime such as "video/*"
  /// both will yield Some(MimeGroup::Video)
  ///
  /// This fn returns none if the passed string contains "*" in the mime group.
  /// in the group or other invalid values.
  ///
  pub fn parse<T: AsRef<str>>(value: T) -> Option<Self> {
    let mut value = value.as_ref();
    if let Some((group, _)) = value.split_once("/") {
      value = group;
    }

    for char in value.bytes() {
      if !check_header_byte(char) {
        return None;
      }
    }

    Some(match value {
      "font" => TiiMimeGroup::Font,
      "application" => TiiMimeGroup::Application,
      "image" => TiiMimeGroup::Image,
      "video" => TiiMimeGroup::Video,
      "audio" => TiiMimeGroup::Audio,
      "text" => TiiMimeGroup::Text,
      _ => TiiMimeGroup::Other(value.to_string()),
    })
  }

  /// returns a static array over all well known mime groups.
  #[must_use]
  pub const fn well_known() -> &'static [TiiMimeGroup] {
    WELL_KNOWN_GROUPS
  }

  /// returns true if this is a well known http mime group.
  #[must_use]
  pub const fn is_well_known(&self) -> bool {
    !matches!(self, Self::Other(_))
  }

  /// returns true if this is a custom http mime group.
  #[must_use]
  pub const fn is_custom(&self) -> bool {
    matches!(self, Self::Other(_))
  }

  /// Returns a static str of the mime group or None if the mime type is heap allocated.
  pub const fn well_known_str(&self) -> Option<&'static str> {
    Some(match self {
      TiiMimeGroup::Font => "font",
      TiiMimeGroup::Application => "application",
      TiiMimeGroup::Image => "image",
      TiiMimeGroup::Video => "video",
      TiiMimeGroup::Audio => "audio",
      TiiMimeGroup::Text => "text",
      TiiMimeGroup::Other(_) => return None,
    })
  }

  /// returns the str name of the mime group.
  /// This name can be fed back into parse to get the equivalent enum of self.
  pub fn as_str(&self) -> &str {
    match self {
      TiiMimeGroup::Font => "font",
      TiiMimeGroup::Application => "application",
      TiiMimeGroup::Image => "image",
      TiiMimeGroup::Video => "video",
      TiiMimeGroup::Audio => "audio",
      TiiMimeGroup::Text => "text",
      TiiMimeGroup::Other(o) => o.as_str(),
    }
  }
}

/// Represents a MIME type as used in the `Content-Type` header.
///
/// # This list is not complete.
/// If you are missing a type then create a PR.
///
/// All PR's for types found on IANA's mime list will always be accepted.
///
/// All PR's for other types will be accepted if the file type is reasonably common
/// and the suggested mime type can found on the internet.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[non_exhaustive]
pub enum TiiMimeType {
  ///////////////////////////////////////// FONT
  /// font/ttf
  FontTtf,
  /// font/otf
  FontOtf,
  /// font/woff
  FontWoff,
  /// font/woff2
  FontWoff2,

  ////////////////////////////////////// Application
  /// application/x-abiword
  ApplicationAbiWord,

  /// application/x-freearc
  ApplicationFreeArc,

  /// application/vnd.amazon.ebook
  ApplicationAmazonEbook,

  /// application/x-bzip
  ApplicationBzip,

  /// application/x-bzip2
  ApplicationBzip2,

  /// application/x-cdf
  ApplicationCDAudio,

  /// application/x-csh
  ApplicationCShell,

  /// application/msword
  ApplicationMicrosoftWord,

  /// application/vnd.openxmlformats-officedocument.wordprocessingml.document
  ApplicationMicrosoftWordXml,

  /// application/vnd.ms-fontobject
  ApplicationMicrosoftFont,

  /// application/epub+zip
  ApplicationEpub,

  /// application/gzip IANA
  /// application/x-gzip Microsoft
  ApplicationGzip,

  /// application/java-archive
  ApplicationJar,

  /// application/x-java-class
  ApplicationJavaClass,

  /// application/octet-stream
  ApplicationOctetStream,
  /// application/json
  ApplicationJson,

  /// application/ld+json
  ApplicationJsonLd,

  /// application/yaml
  ApplicationYaml,

  /// application/x-lua
  TextLua,

  /// application/x-lua-bytecode
  ApplicationLuaBytecode,

  /// application/pdf
  ApplicationPdf,
  /// application/zip
  ApplicationZip,

  /// application/vnd.apple.installer+xml
  ApplicationAppleInstallerPackage,

  /// application/vnd.oasis.opendocument.presentation
  ApplicationOpenDocumentPresentation,

  /// application/vnd.oasis.opendocument.spreadsheet
  ApplicationOpenDocumentSpreadsheet,

  /// application/vnd.oasis.opendocument.text
  ApplicationOpenDocumentText,

  /// application/ogg
  ApplicationOgg,

  /// application/x-httpd-php
  ApplicationPhp,

  /// application/vnd.ms-powerpoint
  ApplicationMicrosoftPowerpoint,

  /// application/vnd.openxmlformats-officedocument.presentationml.presentation
  ApplicationMicrosoftPowerpointXml,

  /// application/vnd.rar
  ApplicationRar,

  /// application/rtf
  ApplicationRichText,

  /// application/x-sh
  ApplicationBourneShell,

  /// application/x-tar
  ApplicationTapeArchive,

  /// application/vnd.visio
  ApplicationMicrosoftVisio,

  /// application/xhtml+xml
  ApplicationXHtml,

  /// application/vnd.ms-excel
  ApplicationMicrosoftExcel,

  /// application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
  ApplicationMicrosoftExcelXml,

  /// application/xml
  /// text/xml
  ApplicationXml,

  /// application/vnd.mozilla.xul+xml
  ApplicationXul,

  /// application/dicom
  ApplicationDicom,

  /// application/x-7z-compressed
  Application7Zip,

  /// application/x-xz
  ApplicationXz,

  /// application/wasm
  ApplicationWasm,

  ////////////////////////////////////// VIDEO
  /// video/mp4
  VideoMp4,
  /// video/ogg
  VideoOgg,
  /// video/webm
  VideoWebm,
  /// video/x-msvideo
  VideoAvi,

  /// video/mpeg
  VideoMpeg,

  /// video/mp2t
  VideoMpegTransportStream,

  /// audio/3gpp
  Video3gpp,

  /// audio/3gpp2
  Video3gpp2,

  ///////////////////////////////////// Image animated and not
  /// image/bmp
  ImageBmp,

  /// image/gif
  ImageGif,

  /// image/jpeg
  ImageJpeg,

  /// image/avif
  ImageAvif,

  /// image/png
  ImagePng,

  /// image/apng
  ImageApng,

  /// image/webp
  ImageWebp,
  /// image/svg+xml
  ImageSvg,
  /// image/vnd.microsoft.icon
  ImageIcon,

  /// image/tiff
  ImageTiff,

  ///////////////////////////////////// AUDIO
  /// audio/aac
  AudioAac,

  /// audio/midi
  /// audio/x-midi
  AudioMidi,

  /// audio/mpeg
  AudioMpeg,

  /// audio/ogg
  AudioOgg,

  /// audio/wav
  AudioWaveform,

  /// audio/webm
  AudioWebm,

  /// audio/3gpp
  Audio3gpp,

  /// audio/3gpp2
  Audio3gpp2,

  //////////////////////////////////// Text documents
  /// text/css
  TextCss,
  /// text/html
  TextHtml,
  /// text/javascript
  TextJavaScript,
  /// text/plain
  TextPlain,
  /// text/csv
  TextCsv,
  /// text/calendar
  TextCalendar,

  ///Anything else
  Other(TiiMimeGroup, String),
}

impl AsRef<TiiMimeType> for TiiMimeType {
  fn as_ref(&self) -> &TiiMimeType {
    self
  }
}

const WELL_KNOWN_TYPES: &[TiiMimeType] = &[
  TiiMimeType::FontTtf,
  TiiMimeType::FontOtf,
  TiiMimeType::FontWoff,
  TiiMimeType::FontWoff2,
  TiiMimeType::ApplicationAbiWord,
  TiiMimeType::ApplicationFreeArc,
  TiiMimeType::ApplicationAmazonEbook,
  TiiMimeType::ApplicationBzip,
  TiiMimeType::ApplicationBzip2,
  TiiMimeType::ApplicationCDAudio,
  TiiMimeType::ApplicationCShell,
  TiiMimeType::ApplicationMicrosoftWord,
  TiiMimeType::ApplicationMicrosoftWordXml,
  TiiMimeType::ApplicationMicrosoftFont,
  TiiMimeType::ApplicationEpub,
  TiiMimeType::ApplicationGzip,
  TiiMimeType::ApplicationJar,
  TiiMimeType::ApplicationJavaClass,
  TiiMimeType::ApplicationOctetStream,
  TiiMimeType::ApplicationJson,
  TiiMimeType::ApplicationJsonLd,
  TiiMimeType::ApplicationPdf,
  TiiMimeType::ApplicationZip,
  TiiMimeType::ApplicationAppleInstallerPackage,
  TiiMimeType::ApplicationOpenDocumentPresentation,
  TiiMimeType::ApplicationOpenDocumentSpreadsheet,
  TiiMimeType::ApplicationOpenDocumentText,
  TiiMimeType::ApplicationOgg,
  TiiMimeType::ApplicationPhp,
  TiiMimeType::ApplicationMicrosoftPowerpoint,
  TiiMimeType::ApplicationMicrosoftPowerpointXml,
  TiiMimeType::ApplicationRar,
  TiiMimeType::ApplicationRichText,
  TiiMimeType::ApplicationBourneShell,
  TiiMimeType::ApplicationTapeArchive,
  TiiMimeType::ApplicationMicrosoftVisio,
  TiiMimeType::ApplicationXHtml,
  TiiMimeType::ApplicationMicrosoftExcel,
  TiiMimeType::ApplicationMicrosoftExcelXml,
  TiiMimeType::ApplicationXml,
  TiiMimeType::ApplicationXul,
  TiiMimeType::ApplicationDicom,
  TiiMimeType::Application7Zip,
  TiiMimeType::ApplicationWasm,
  TiiMimeType::VideoMp4,
  TiiMimeType::VideoOgg,
  TiiMimeType::VideoWebm,
  TiiMimeType::VideoAvi,
  TiiMimeType::VideoMpeg,
  TiiMimeType::VideoMpegTransportStream,
  TiiMimeType::Video3gpp,
  TiiMimeType::Video3gpp2,
  TiiMimeType::ImageBmp,
  TiiMimeType::ImageGif,
  TiiMimeType::ImageJpeg,
  TiiMimeType::ImageAvif,
  TiiMimeType::ImagePng,
  TiiMimeType::ImageApng,
  TiiMimeType::ImageWebp,
  TiiMimeType::ImageSvg,
  TiiMimeType::ImageIcon,
  TiiMimeType::ImageTiff,
  TiiMimeType::AudioAac,
  TiiMimeType::AudioMidi,
  TiiMimeType::AudioMpeg,
  TiiMimeType::AudioOgg,
  TiiMimeType::AudioWaveform,
  TiiMimeType::AudioWebm,
  TiiMimeType::Audio3gpp,
  TiiMimeType::Audio3gpp2,
  TiiMimeType::TextCss,
  TiiMimeType::TextHtml,
  TiiMimeType::TextJavaScript,
  TiiMimeType::TextPlain,
  TiiMimeType::TextCsv,
  TiiMimeType::TextCalendar,
  TiiMimeType::ApplicationYaml,
  TiiMimeType::TextLua,
  TiiMimeType::ApplicationLuaBytecode,
  TiiMimeType::ApplicationXz,
];

impl TiiMimeType {
  /// Converts from a file extension without the `.` to the enum variant.
  /// If the MIME type cannot be inferred from the extension, returns `MimeType::ApplicationOctetStream`.
  pub fn from_extension(extension: impl AsRef<str>) -> Self {
    //TODO Heap allocation to_ascii_lowercase
    match extension.as_ref().to_ascii_lowercase().as_str() {
      "css" => TiiMimeType::TextCss,
      "html" => TiiMimeType::TextHtml,
      "htm" => TiiMimeType::TextHtml,
      "js" => TiiMimeType::TextJavaScript,
      "mjs" => TiiMimeType::TextJavaScript,
      "txt" => TiiMimeType::TextPlain,
      "bmp" => TiiMimeType::ImageBmp,
      "gif" => TiiMimeType::ImageGif,
      "jpeg" => TiiMimeType::ImageJpeg,
      "jpg" => TiiMimeType::ImageJpeg,
      "png" => TiiMimeType::ImagePng,
      "webp" => TiiMimeType::ImageWebp,
      "svg" => TiiMimeType::ImageSvg,
      "ico" => TiiMimeType::ImageIcon,
      "json" => TiiMimeType::ApplicationJson,
      "pdf" => TiiMimeType::ApplicationPdf,
      "zip" => TiiMimeType::ApplicationZip,
      "mp4" => TiiMimeType::VideoMp4,
      "ogv" => TiiMimeType::VideoOgg,
      "webm" => TiiMimeType::VideoWebm,
      "ttf" => TiiMimeType::FontTtf,
      "otf" => TiiMimeType::FontOtf,
      "woff" => TiiMimeType::FontWoff,
      "woff2" => TiiMimeType::FontWoff2,
      "abw" => TiiMimeType::ApplicationAbiWord,
      "arc" => TiiMimeType::ApplicationFreeArc,
      "azw" => TiiMimeType::ApplicationAmazonEbook,
      "bz" => TiiMimeType::ApplicationBzip,
      "bz2" => TiiMimeType::ApplicationBzip2,
      "cda" => TiiMimeType::ApplicationCDAudio,
      "csh" => TiiMimeType::ApplicationCShell,
      "doc" => TiiMimeType::ApplicationMicrosoftWord,
      "docx" => TiiMimeType::ApplicationMicrosoftWordXml,
      "eot" => TiiMimeType::ApplicationMicrosoftFont,
      "epub" => TiiMimeType::ApplicationEpub,
      "gz" => TiiMimeType::ApplicationGzip,
      "jar" => TiiMimeType::ApplicationJar,
      "class" => TiiMimeType::ApplicationJavaClass,
      "bin" => TiiMimeType::ApplicationOctetStream,
      "jsonld" => TiiMimeType::ApplicationJsonLd,
      "mpkg" => TiiMimeType::ApplicationAppleInstallerPackage,
      "odp" => TiiMimeType::ApplicationOpenDocumentPresentation,
      "ods" => TiiMimeType::ApplicationOpenDocumentSpreadsheet,
      "odt" => TiiMimeType::ApplicationOpenDocumentText,
      "ogx" => TiiMimeType::ApplicationOgg,
      "php" => TiiMimeType::ApplicationPhp,
      "ppt" => TiiMimeType::ApplicationMicrosoftPowerpoint,
      "pptx" => TiiMimeType::ApplicationMicrosoftPowerpointXml,
      "rar" => TiiMimeType::ApplicationRar,
      "rtf" => TiiMimeType::ApplicationRichText,
      "sh" => TiiMimeType::ApplicationBourneShell,
      "tar" => TiiMimeType::ApplicationTapeArchive,
      "vsd" => TiiMimeType::ApplicationMicrosoftVisio,
      "xhtml" => TiiMimeType::ApplicationXHtml,
      "xls" => TiiMimeType::ApplicationMicrosoftExcel,
      "xlsx" => TiiMimeType::ApplicationMicrosoftExcelXml,
      "xml" => TiiMimeType::ApplicationXml,
      "xul" => TiiMimeType::ApplicationXul,
      "dcm" => TiiMimeType::ApplicationDicom,
      "7z" => TiiMimeType::Application7Zip,
      "wasm" => TiiMimeType::ApplicationWasm,
      "avi" => TiiMimeType::VideoAvi,
      "mpeg" => TiiMimeType::VideoMpeg,
      "ts" => TiiMimeType::VideoMpegTransportStream,
      "3gp" => TiiMimeType::Video3gpp,
      "3g2" => TiiMimeType::Video3gpp2,
      "avif" => TiiMimeType::ImageAvif,
      "apng" => TiiMimeType::ImageApng,
      "tif" => TiiMimeType::ImageTiff,
      "aac" => TiiMimeType::AudioAac,
      "mid" => TiiMimeType::AudioMidi,
      "mp3" => TiiMimeType::AudioMpeg,
      "oga" => TiiMimeType::AudioOgg,
      "wav" => TiiMimeType::AudioWaveform,
      "weba" => TiiMimeType::AudioWebm,
      "csv" => TiiMimeType::TextCsv,
      "cal" => TiiMimeType::TextCalendar,
      "yaml" | "yml" => TiiMimeType::ApplicationYaml,
      "lua" => TiiMimeType::TextLua,
      "luac" => TiiMimeType::ApplicationLuaBytecode,
      "xz" => TiiMimeType::ApplicationXz,
      _ => TiiMimeType::ApplicationOctetStream,
    }
  }

  /// returns the file extension that is most likely correct for the given file type.
  /// For mime types where this is not clear "bin" is returned.
  #[must_use]
  pub const fn extension(&self) -> &'static str {
    match self {
      TiiMimeType::FontTtf => "ttf",
      TiiMimeType::FontOtf => "otf",
      TiiMimeType::FontWoff => "woff",
      TiiMimeType::FontWoff2 => "woff2",
      TiiMimeType::ApplicationAbiWord => "abw",
      TiiMimeType::ApplicationFreeArc => "arc",
      TiiMimeType::ApplicationAmazonEbook => "azw",
      TiiMimeType::ApplicationBzip => "bz",
      TiiMimeType::ApplicationBzip2 => "bz2",
      TiiMimeType::ApplicationCDAudio => "cda",
      TiiMimeType::ApplicationCShell => "csh",
      TiiMimeType::ApplicationMicrosoftWord => "doc",
      TiiMimeType::ApplicationMicrosoftWordXml => "docx",
      TiiMimeType::ApplicationMicrosoftFont => "eot",
      TiiMimeType::ApplicationEpub => "epub",
      TiiMimeType::ApplicationGzip => "gz",
      TiiMimeType::ApplicationJar => "jar",
      TiiMimeType::ApplicationJavaClass => "class",
      TiiMimeType::ApplicationOctetStream => "bin",
      TiiMimeType::ApplicationJson => "json",
      TiiMimeType::ApplicationJsonLd => "jsonld",
      TiiMimeType::ApplicationPdf => "pdf",
      TiiMimeType::ApplicationZip => "zip",
      TiiMimeType::ApplicationAppleInstallerPackage => "mpkg",
      TiiMimeType::ApplicationOpenDocumentPresentation => "odp",
      TiiMimeType::ApplicationOpenDocumentSpreadsheet => "ods",
      TiiMimeType::ApplicationOpenDocumentText => "odt",
      TiiMimeType::ApplicationOgg => "ogx",
      TiiMimeType::ApplicationPhp => "php",
      TiiMimeType::ApplicationMicrosoftPowerpoint => "ppt",
      TiiMimeType::ApplicationMicrosoftPowerpointXml => "pptx",
      TiiMimeType::ApplicationRar => "rar",
      TiiMimeType::ApplicationRichText => "rtf",
      TiiMimeType::ApplicationBourneShell => "sh",
      TiiMimeType::ApplicationTapeArchive => "tar",
      TiiMimeType::ApplicationMicrosoftVisio => "vsd",
      TiiMimeType::ApplicationXHtml => "xhtml",
      TiiMimeType::ApplicationMicrosoftExcel => "xls",
      TiiMimeType::ApplicationMicrosoftExcelXml => "xlsx",
      TiiMimeType::ApplicationXml => "xml",
      TiiMimeType::ApplicationXul => "xul",
      TiiMimeType::ApplicationDicom => "dcm",
      TiiMimeType::Application7Zip => "7z",
      TiiMimeType::ApplicationWasm => "wasm",
      TiiMimeType::VideoMp4 => "mp4",
      TiiMimeType::VideoOgg => "ogv",
      TiiMimeType::VideoWebm => "webm",
      TiiMimeType::VideoAvi => "avi",
      TiiMimeType::VideoMpeg => "mpeg",
      TiiMimeType::VideoMpegTransportStream => "ts",
      TiiMimeType::Video3gpp => "3gp",
      TiiMimeType::Video3gpp2 => "3g2",
      TiiMimeType::ImageBmp => "bmp",
      TiiMimeType::ImageGif => "gif",
      TiiMimeType::ImageJpeg => "jpg",
      TiiMimeType::ImageAvif => "avif",
      TiiMimeType::ImagePng => "png",
      TiiMimeType::ImageApng => "apng",
      TiiMimeType::ImageWebp => "webp",
      TiiMimeType::ImageSvg => "svg",
      TiiMimeType::ImageIcon => "ico",
      TiiMimeType::ImageTiff => "tif",
      TiiMimeType::AudioAac => "aac",
      TiiMimeType::AudioMidi => "mid",
      TiiMimeType::AudioMpeg => "mp3",
      TiiMimeType::AudioOgg => "oga",
      TiiMimeType::AudioWaveform => "wav",
      TiiMimeType::AudioWebm => "weba",
      TiiMimeType::Audio3gpp => "3gp",
      TiiMimeType::Audio3gpp2 => "3g2",
      TiiMimeType::TextCss => "css",
      TiiMimeType::TextHtml => "html",
      TiiMimeType::TextJavaScript => "js",
      TiiMimeType::TextPlain => "txt",
      TiiMimeType::TextCsv => "csv",
      TiiMimeType::TextCalendar => "cal",
      TiiMimeType::ApplicationYaml => "yaml",
      TiiMimeType::TextLua => "lua",
      TiiMimeType::ApplicationLuaBytecode => "luac",
      TiiMimeType::ApplicationXz => "xz",
      TiiMimeType::Other(_, _) => "bin",
    }
  }

  /// returns the MimeGroup of this mime type.
  pub const fn mime_group(&self) -> &TiiMimeGroup {
    match self {
      TiiMimeType::FontTtf => &TiiMimeGroup::Font,
      TiiMimeType::FontOtf => &TiiMimeGroup::Font,
      TiiMimeType::FontWoff => &TiiMimeGroup::Font,
      TiiMimeType::FontWoff2 => &TiiMimeGroup::Font,
      TiiMimeType::ApplicationAbiWord => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationFreeArc => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationAmazonEbook => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationBzip => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationBzip2 => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationCDAudio => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationCShell => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftWord => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftWordXml => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftFont => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationEpub => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationGzip => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationJar => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationJavaClass => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationOctetStream => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationJson => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationJsonLd => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationYaml => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationLuaBytecode => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationPdf => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationZip => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationAppleInstallerPackage => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationOpenDocumentPresentation => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationOpenDocumentSpreadsheet => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationOpenDocumentText => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationOgg => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationPhp => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftPowerpoint => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftPowerpointXml => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationRar => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationRichText => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationBourneShell => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationTapeArchive => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftVisio => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationXHtml => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftExcel => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationMicrosoftExcelXml => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationXml => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationXul => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationDicom => &TiiMimeGroup::Application,
      TiiMimeType::Application7Zip => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationXz => &TiiMimeGroup::Application,
      TiiMimeType::ApplicationWasm => &TiiMimeGroup::Application,
      TiiMimeType::VideoMp4 => &TiiMimeGroup::Video,
      TiiMimeType::VideoOgg => &TiiMimeGroup::Video,
      TiiMimeType::VideoWebm => &TiiMimeGroup::Video,
      TiiMimeType::VideoAvi => &TiiMimeGroup::Video,
      TiiMimeType::VideoMpeg => &TiiMimeGroup::Video,
      TiiMimeType::VideoMpegTransportStream => &TiiMimeGroup::Video,
      TiiMimeType::Video3gpp => &TiiMimeGroup::Video,
      TiiMimeType::Video3gpp2 => &TiiMimeGroup::Video,
      TiiMimeType::ImageBmp => &TiiMimeGroup::Image,
      TiiMimeType::ImageGif => &TiiMimeGroup::Image,
      TiiMimeType::ImageJpeg => &TiiMimeGroup::Image,
      TiiMimeType::ImageAvif => &TiiMimeGroup::Image,
      TiiMimeType::ImagePng => &TiiMimeGroup::Image,
      TiiMimeType::ImageApng => &TiiMimeGroup::Image,
      TiiMimeType::ImageWebp => &TiiMimeGroup::Image,
      TiiMimeType::ImageSvg => &TiiMimeGroup::Image,
      TiiMimeType::ImageIcon => &TiiMimeGroup::Image,
      TiiMimeType::ImageTiff => &TiiMimeGroup::Image,
      TiiMimeType::AudioAac => &TiiMimeGroup::Audio,
      TiiMimeType::AudioMidi => &TiiMimeGroup::Audio,
      TiiMimeType::AudioMpeg => &TiiMimeGroup::Audio,
      TiiMimeType::AudioOgg => &TiiMimeGroup::Audio,
      TiiMimeType::AudioWaveform => &TiiMimeGroup::Audio,
      TiiMimeType::AudioWebm => &TiiMimeGroup::Audio,
      TiiMimeType::Audio3gpp => &TiiMimeGroup::Audio,
      TiiMimeType::Audio3gpp2 => &TiiMimeGroup::Audio,
      TiiMimeType::TextCss => &TiiMimeGroup::Text,
      TiiMimeType::TextHtml => &TiiMimeGroup::Text,
      TiiMimeType::TextJavaScript => &TiiMimeGroup::Text,
      TiiMimeType::TextLua => &TiiMimeGroup::Text,
      TiiMimeType::TextPlain => &TiiMimeGroup::Text,
      TiiMimeType::TextCsv => &TiiMimeGroup::Text,
      TiiMimeType::TextCalendar => &TiiMimeGroup::Text,
      TiiMimeType::Other(group, _) => group,
    }
  }

  /// Does this mime type have an extension that is only used by this mime type and not shared with any other well known mime type?
  /// Types where this returns true cannot be relied upon to work with `MimeType::from_extension`
  pub const fn has_unique_known_extension(&self) -> bool {
    match self {
      TiiMimeType::Video3gpp2 | TiiMimeType::Audio3gpp2 => false, //3g2 is shared
      TiiMimeType::Video3gpp | TiiMimeType::Audio3gpp => false,   //3gp is shared
      TiiMimeType::Other(_, _) => false, //We don't know what the extension even is.
      _ => true,
    }
  }

  /// returns a static slice that contains all well known mime types.
  #[must_use]
  pub const fn well_known() -> &'static [TiiMimeType] {
    WELL_KNOWN_TYPES
  }

  /// returns true if this is a well known http mime type.
  #[must_use]
  pub const fn is_well_known(&self) -> bool {
    !matches!(self, TiiMimeType::Other(_, _))
  }

  /// returns true if this is a custom http mime type.
  #[must_use]
  pub const fn is_custom(&self) -> bool {
    matches!(self, Self::Other(_, _))
  }

  /// Returns a static str of the mime type or None if the mime type is heap allocated.
  pub const fn well_known_str(&self) -> Option<&'static str> {
    Some(match self {
      TiiMimeType::TextCss => "text/css",
      TiiMimeType::TextHtml => "text/html",
      TiiMimeType::TextJavaScript => "text/javascript",
      TiiMimeType::TextPlain => "text/plain",
      TiiMimeType::ImageBmp => "image/bmp",
      TiiMimeType::ImageGif => "image/gif",
      TiiMimeType::ImageJpeg => "image/jpeg",
      TiiMimeType::ImagePng => "image/png",
      TiiMimeType::ImageWebp => "image/webp",
      TiiMimeType::ImageSvg => "image/svg+xml",
      TiiMimeType::ImageIcon => "image/vnd.microsoft.icon",
      TiiMimeType::ApplicationOctetStream => "application/octet-stream",
      TiiMimeType::ApplicationJson => "application/json",
      TiiMimeType::ApplicationPdf => "application/pdf",
      TiiMimeType::ApplicationZip => "application/zip",
      TiiMimeType::VideoMp4 => "video/mp4",
      TiiMimeType::VideoOgg => "video/ogg",
      TiiMimeType::VideoWebm => "video/webm",
      TiiMimeType::FontTtf => "font/ttf",
      TiiMimeType::FontOtf => "font/otf",
      TiiMimeType::FontWoff => "font/woff",
      TiiMimeType::FontWoff2 => "font/woff2",
      TiiMimeType::ApplicationAbiWord => "application/x-abiword",
      TiiMimeType::ApplicationFreeArc => "application/x-freearc",
      TiiMimeType::ApplicationAmazonEbook => "application/vnd.amazon.ebook",
      TiiMimeType::ApplicationBzip => "application/x-bzip",
      TiiMimeType::ApplicationBzip2 => "application/x-bzip2",
      TiiMimeType::ApplicationCDAudio => "application/x-cdf",
      TiiMimeType::ApplicationCShell => "application/x-csh",
      TiiMimeType::ApplicationMicrosoftWord => "application/msword",
      TiiMimeType::ApplicationMicrosoftWordXml => {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
      }
      TiiMimeType::ApplicationMicrosoftFont => "application/vnd.ms-fontobject",
      TiiMimeType::ApplicationEpub => "application/epub+zip",
      TiiMimeType::ApplicationGzip => "application/gzip",
      TiiMimeType::ApplicationJar => "application/java-archive",
      TiiMimeType::ApplicationJavaClass => "application/x-java-class",
      TiiMimeType::ApplicationJsonLd => "application/ld+json",
      TiiMimeType::ApplicationAppleInstallerPackage => "application/vnd.apple.installer+xml",
      TiiMimeType::ApplicationOpenDocumentPresentation => {
        "application/vnd.oasis.opendocument.presentation"
      }
      TiiMimeType::ApplicationOpenDocumentSpreadsheet => {
        "application/vnd.oasis.opendocument.spreadsheet"
      }
      TiiMimeType::ApplicationOpenDocumentText => "application/vnd.oasis.opendocument.text",
      TiiMimeType::ApplicationOgg => "application/ogg",
      TiiMimeType::ApplicationPhp => "application/x-httpd-php",
      TiiMimeType::ApplicationMicrosoftPowerpoint => "application/vnd.ms-powerpoint",
      TiiMimeType::ApplicationMicrosoftPowerpointXml => {
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
      }
      TiiMimeType::ApplicationRar => "application/vnd.rar",
      TiiMimeType::ApplicationRichText => "application/rtf",
      TiiMimeType::ApplicationBourneShell => "application/x-sh",
      TiiMimeType::ApplicationTapeArchive => "application/x-tar",
      TiiMimeType::ApplicationMicrosoftVisio => "application/vnd.visio",
      TiiMimeType::ApplicationXHtml => "application/xhtml+xml",
      TiiMimeType::ApplicationMicrosoftExcel => "application/vnd.ms-excel",
      TiiMimeType::ApplicationMicrosoftExcelXml => {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      }
      TiiMimeType::ApplicationXml => "application/xml",
      TiiMimeType::ApplicationXul => "application/vnd.mozilla.xul+xml",
      TiiMimeType::ApplicationDicom => "application/dicom",
      TiiMimeType::Application7Zip => "application/x-7z-compressed",
      TiiMimeType::ApplicationWasm => "application/wasm",
      TiiMimeType::VideoAvi => "video/x-msvideo",
      TiiMimeType::VideoMpeg => "video/mpeg",
      TiiMimeType::VideoMpegTransportStream => "video/mp2t",
      TiiMimeType::Video3gpp => "video/3gpp",
      TiiMimeType::Video3gpp2 => "video/3gpp2",
      TiiMimeType::ImageAvif => "image/avif",
      TiiMimeType::ImageApng => "image/apng",
      TiiMimeType::ImageTiff => "image/tiff",
      TiiMimeType::AudioAac => "audio/aac",
      TiiMimeType::AudioMidi => "audio/midi",
      TiiMimeType::AudioMpeg => "audio/mpeg",
      TiiMimeType::AudioOgg => "audio/ogg",
      TiiMimeType::AudioWaveform => "audio/wav",
      TiiMimeType::AudioWebm => "audio/webm",
      TiiMimeType::Audio3gpp => "audio/3gpp",
      TiiMimeType::Audio3gpp2 => "audio/3gpp2",
      TiiMimeType::TextCsv => "text/csv",
      TiiMimeType::TextCalendar => "text/calendar",
      TiiMimeType::ApplicationYaml => "application/yaml",
      TiiMimeType::TextLua => "text/x-lua",
      TiiMimeType::ApplicationLuaBytecode => "application/x-lua-bytecode",
      TiiMimeType::ApplicationXz => "application/x-xz",
      TiiMimeType::Other(_, _) => return None,
    })
  }

  /// returns the &str representation of the mime type.
  pub fn as_str(&self) -> &str {
    match self {
      TiiMimeType::TextCss => "text/css",
      TiiMimeType::TextHtml => "text/html",
      TiiMimeType::TextJavaScript => "text/javascript",
      TiiMimeType::TextPlain => "text/plain",
      TiiMimeType::ImageBmp => "image/bmp",
      TiiMimeType::ImageGif => "image/gif",
      TiiMimeType::ImageJpeg => "image/jpeg",
      TiiMimeType::ImagePng => "image/png",
      TiiMimeType::ImageWebp => "image/webp",
      TiiMimeType::ImageSvg => "image/svg+xml",
      TiiMimeType::ImageIcon => "image/vnd.microsoft.icon",
      TiiMimeType::ApplicationOctetStream => "application/octet-stream",
      TiiMimeType::ApplicationJson => "application/json",
      TiiMimeType::ApplicationPdf => "application/pdf",
      TiiMimeType::ApplicationZip => "application/zip",
      TiiMimeType::VideoMp4 => "video/mp4",
      TiiMimeType::VideoOgg => "video/ogg",
      TiiMimeType::VideoWebm => "video/webm",
      TiiMimeType::FontTtf => "font/ttf",
      TiiMimeType::FontOtf => "font/otf",
      TiiMimeType::FontWoff => "font/woff",
      TiiMimeType::FontWoff2 => "font/woff2",
      TiiMimeType::ApplicationAbiWord => "application/x-abiword",
      TiiMimeType::ApplicationFreeArc => "application/x-freearc",
      TiiMimeType::ApplicationAmazonEbook => "application/vnd.amazon.ebook",
      TiiMimeType::ApplicationBzip => "application/x-bzip",
      TiiMimeType::ApplicationBzip2 => "application/x-bzip2",
      TiiMimeType::ApplicationCDAudio => "application/x-cdf",
      TiiMimeType::ApplicationCShell => "application/x-csh",
      TiiMimeType::ApplicationMicrosoftWord => "application/msword",
      TiiMimeType::ApplicationMicrosoftWordXml => {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
      }
      TiiMimeType::ApplicationMicrosoftFont => "application/vnd.ms-fontobject",
      TiiMimeType::ApplicationEpub => "application/epub+zip",
      TiiMimeType::ApplicationGzip => "application/gzip",
      TiiMimeType::ApplicationJar => "application/java-archive",
      TiiMimeType::ApplicationJavaClass => "application/x-java-class",
      TiiMimeType::ApplicationJsonLd => "application/ld+json",
      TiiMimeType::ApplicationAppleInstallerPackage => "application/vnd.apple.installer+xml",
      TiiMimeType::ApplicationOpenDocumentPresentation => {
        "application/vnd.oasis.opendocument.presentation"
      }
      TiiMimeType::ApplicationOpenDocumentSpreadsheet => {
        "application/vnd.oasis.opendocument.spreadsheet"
      }
      TiiMimeType::ApplicationOpenDocumentText => "application/vnd.oasis.opendocument.text",
      TiiMimeType::ApplicationOgg => "application/ogg",
      TiiMimeType::ApplicationPhp => "application/x-httpd-php",
      TiiMimeType::ApplicationMicrosoftPowerpoint => "application/vnd.ms-powerpoint",
      TiiMimeType::ApplicationMicrosoftPowerpointXml => {
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
      }
      TiiMimeType::ApplicationRar => "application/vnd.rar",
      TiiMimeType::ApplicationRichText => "application/rtf",
      TiiMimeType::ApplicationBourneShell => "application/x-sh",
      TiiMimeType::ApplicationTapeArchive => "application/x-tar",
      TiiMimeType::ApplicationMicrosoftVisio => "application/vnd.visio",
      TiiMimeType::ApplicationXHtml => "application/xhtml+xml",
      TiiMimeType::ApplicationMicrosoftExcel => "application/vnd.ms-excel",
      TiiMimeType::ApplicationMicrosoftExcelXml => {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      }
      TiiMimeType::ApplicationXml => "application/xml",
      TiiMimeType::ApplicationXul => "application/vnd.mozilla.xul+xml",
      TiiMimeType::ApplicationDicom => "application/dicom",
      TiiMimeType::Application7Zip => "application/x-7z-compressed",
      TiiMimeType::ApplicationWasm => "application/wasm",
      TiiMimeType::VideoAvi => "video/x-msvideo",
      TiiMimeType::VideoMpeg => "video/mpeg",
      TiiMimeType::VideoMpegTransportStream => "video/mp2t",
      TiiMimeType::Video3gpp => "video/3gpp",
      TiiMimeType::Video3gpp2 => "video/3gpp2",
      TiiMimeType::ImageAvif => "image/avif",
      TiiMimeType::ImageApng => "image/apng",
      TiiMimeType::ImageTiff => "image/tiff",
      TiiMimeType::AudioAac => "audio/aac",
      TiiMimeType::AudioMidi => "audio/midi",
      TiiMimeType::AudioMpeg => "audio/mpeg",
      TiiMimeType::AudioOgg => "audio/ogg",
      TiiMimeType::AudioWaveform => "audio/wav",
      TiiMimeType::AudioWebm => "audio/webm",
      TiiMimeType::Audio3gpp => "audio/3gpp",
      TiiMimeType::Audio3gpp2 => "audio/3gpp2",
      TiiMimeType::TextCsv => "text/csv",
      TiiMimeType::TextCalendar => "text/calendar",
      TiiMimeType::ApplicationYaml => "application/yaml",
      TiiMimeType::TextLua => "text/x-lua",
      TiiMimeType::ApplicationLuaBytecode => "application/x-lua-bytecode",
      TiiMimeType::ApplicationXz => "application/x-xz",
      TiiMimeType::Other(_, data) => data.as_str(),
    }
  }

  /// This fn parses the mime type and assumes that its in the format of a valid Content-Type header.
  pub fn parse_from_content_type_header<T: AsRef<str>>(value: T) -> Option<Self> {
    Self::parse(unwrap_some(value.as_ref().split(";").next())) //strips ; charset=utf-8 or ; boundry=..., which we don't care about here.
  }

  /// Parses the string value and returns a mime type.
  /// Returns none for invalid mime types.
  pub fn parse<T: AsRef<str>>(value: T) -> Option<Self> {
    Some(match value.as_ref() {
      "text/css" => TiiMimeType::TextCss,
      "text/html" => TiiMimeType::TextHtml,
      "text/javascript" => TiiMimeType::TextJavaScript,
      "text/plain" => TiiMimeType::TextPlain,
      "image/bmp" => TiiMimeType::ImageBmp,
      "image/gif" => TiiMimeType::ImageGif,
      "image/jpeg" => TiiMimeType::ImageJpeg,
      "image/png" => TiiMimeType::ImagePng,
      "image/webp" => TiiMimeType::ImageWebp,
      "image/svg+xml" => TiiMimeType::ImageSvg,
      "image/vnd.microsoft.icon" => TiiMimeType::ImageIcon,
      "application/octet-stream" => TiiMimeType::ApplicationOctetStream,
      "application/json" => TiiMimeType::ApplicationJson,
      "application/pdf" => TiiMimeType::ApplicationPdf,
      "application/zip" => TiiMimeType::ApplicationZip,
      "video/mp4" => TiiMimeType::VideoMp4,
      "video/ogg" => TiiMimeType::VideoOgg,
      "video/webm" => TiiMimeType::VideoWebm,
      "font/ttf" => TiiMimeType::FontTtf,
      "font/otf" => TiiMimeType::FontOtf,
      "font/woff" => TiiMimeType::FontWoff,
      "font/woff2" => TiiMimeType::FontWoff2,
      "application/x-abiword" => TiiMimeType::ApplicationAbiWord,
      "application/x-freearc" => TiiMimeType::ApplicationFreeArc,
      "application/vnd.amazon.ebook" => TiiMimeType::ApplicationAmazonEbook,
      "application/x-bzip" => TiiMimeType::ApplicationBzip,
      "application/x-bzip2" => TiiMimeType::ApplicationBzip2,
      "application/x-cdf" => TiiMimeType::ApplicationCDAudio,
      "application/x-csh" => TiiMimeType::ApplicationCShell,
      "application/msword" => TiiMimeType::ApplicationMicrosoftWord,
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
        TiiMimeType::ApplicationMicrosoftWordXml
      }
      "application/vnd.ms-fontobject" => TiiMimeType::ApplicationMicrosoftFont,
      "application/epub+zip" => TiiMimeType::ApplicationEpub,
      "application/gzip" => TiiMimeType::ApplicationGzip,
      "application/java-archive" => TiiMimeType::ApplicationJar,
      "application/x-java-class" => TiiMimeType::ApplicationJavaClass,
      "application/ld+json" => TiiMimeType::ApplicationJsonLd,
      "application/vnd.apple.installer+xml" => TiiMimeType::ApplicationAppleInstallerPackage,
      "application/vnd.oasis.opendocument.presentation" => {
        TiiMimeType::ApplicationOpenDocumentPresentation
      }
      "application/vnd.oasis.opendocument.spreadsheet" => {
        TiiMimeType::ApplicationOpenDocumentSpreadsheet
      }
      "application/vnd.oasis.opendocument.text" => TiiMimeType::ApplicationOpenDocumentText,
      "application/ogg" => TiiMimeType::ApplicationOgg,
      "application/x-httpd-php" => TiiMimeType::ApplicationPhp,
      "application/vnd.ms-powerpoint" => TiiMimeType::ApplicationMicrosoftPowerpoint,
      "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
        TiiMimeType::ApplicationMicrosoftPowerpointXml
      }
      "application/vnd.rar" => TiiMimeType::ApplicationRar,
      "application/rtf" => TiiMimeType::ApplicationRichText,
      "application/x-sh" => TiiMimeType::ApplicationBourneShell,
      "application/x-tar" => TiiMimeType::ApplicationTapeArchive,
      "application/vnd.visio" => TiiMimeType::ApplicationMicrosoftVisio,
      "application/xhtml+xml" => TiiMimeType::ApplicationXHtml,
      "application/vnd.ms-excel" => TiiMimeType::ApplicationMicrosoftExcel,
      "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
        TiiMimeType::ApplicationMicrosoftExcelXml
      }
      "application/xml" => TiiMimeType::ApplicationXml,
      "application/vnd.mozilla.xul+xml" => TiiMimeType::ApplicationXul,
      "application/dicom" => TiiMimeType::ApplicationDicom,
      "application/x-7z-compressed" => TiiMimeType::Application7Zip,
      "application/wasm" => TiiMimeType::ApplicationWasm,
      "video/x-msvideo" => TiiMimeType::VideoAvi,
      "video/mpeg" => TiiMimeType::VideoMpeg,
      "video/mp2t" => TiiMimeType::VideoMpegTransportStream,
      "video/3gpp" => TiiMimeType::Video3gpp,
      "video/3gpp2" => TiiMimeType::Video3gpp2,
      "audio/3gpp" => TiiMimeType::Audio3gpp,
      "audio/3gpp2" => TiiMimeType::Audio3gpp2,
      "image/avif" => TiiMimeType::ImageAvif,
      "image/apng" => TiiMimeType::ImageApng,
      "image/tiff" => TiiMimeType::ImageTiff,
      "audio/aac" => TiiMimeType::AudioAac,
      "audio/midi" => TiiMimeType::AudioMidi,
      "audio/mpeg" => TiiMimeType::AudioMpeg,
      "audio/ogg" => TiiMimeType::AudioOgg,
      "audio/wav" => TiiMimeType::AudioWaveform,
      "audio/webm" => TiiMimeType::AudioWebm,
      "text/csv" => TiiMimeType::TextCsv,
      "text/calendar" => TiiMimeType::TextCalendar,
      "application/yaml" => TiiMimeType::ApplicationYaml,
      "text/x-lua" => TiiMimeType::TextLua,
      "application/x-lua-bytecode" => TiiMimeType::ApplicationLuaBytecode,
      "application/x-xz" => TiiMimeType::ApplicationXz,
      other => {
        if other.starts_with('/') || other.ends_with('/') {
          return None;
        }

        let mut found_slash = false;
        for char in other.bytes() {
          if char == b'/' {
            if found_slash {
              return None;
            }
            found_slash = true;
            continue;
          }

          if !check_header_byte(char) {
            return None;
          }
        }

        if !found_slash {
          return None;
        }

        if let Some(grp) = TiiMimeGroup::parse(other) {
          TiiMimeType::Other(grp, other.to_string())
        } else {
          // We already do a superset of validations, this case is impossible.
          crate::util::unreachable()
        }
      }
    })
  }
}

impl Display for TiiMimeType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

const fn check_header_byte(char: u8) -> bool {
  if char <= 31 {
    //Ascii control characters, not allowed here!
    return false;
  }

  if char & 0b1000_0000 != 0 {
    //Multibyte utf-8 not permitted, this must be ascii!
    return false;
  }

  if char.is_ascii_uppercase() {
    // Upper case not permitted. (TODO is this correct? In practice ive only ever seen them lower case.)
    return false;
  }

  //TODO actually lookup the RFC and verify what exact printable characters are permitted here.
  !matches!(
    char,
    b'*'
      | b'('
      | b')'
      | b':'
      | b'<'
      | b'>'
      | b'?'
      | b'@'
      | b'['
      | b']'
      | b'\\'
      | b'{'
      | b'}'
      | 0x7F
  )
}

impl From<TiiMimeType> for TiiAcceptMimeType {
  fn from(value: TiiMimeType) -> Self {
    TiiAcceptMimeType::Specific(value)
  }
}

impl From<&TiiMimeType> for TiiAcceptMimeType {
  fn from(value: &TiiMimeType) -> Self {
    TiiAcceptMimeType::Specific(value.clone())
  }
}

impl From<TiiMimeGroup> for TiiAcceptMimeType {
  fn from(value: TiiMimeGroup) -> Self {
    TiiAcceptMimeType::GroupWildcard(value)
  }
}

impl From<&TiiMimeGroup> for TiiAcceptMimeType {
  fn from(value: &TiiMimeGroup) -> Self {
    TiiAcceptMimeType::GroupWildcard(value.clone())
  }
}

impl From<TiiMimeType> for TiiMimeGroup {
  fn from(value: TiiMimeType) -> Self {
    value.mime_group().clone()
  }
}

impl From<&TiiMimeType> for TiiMimeGroup {
  fn from(value: &TiiMimeType) -> Self {
    value.mime_group().clone()
  }
}

impl From<TiiAcceptQualityMimeType> for TiiAcceptMimeType {
  fn from(value: TiiAcceptQualityMimeType) -> Self {
    value.value
  }
}

#[cfg(test)]
mod tests {
  use crate::http::mime::TiiQValue;

  /// Shutup clippy.
  #[macro_export]
  macro_rules! test_qvalue {
    ($input:expr, $expected:expr) => {
      let q = TiiQValue($input);
      assert_eq!(q.as_str(), $expected);
    };
  }

  #[test]
  fn constutil() {
    // Only covering/testing some edge cases as a sanity check. See the proc macro for the full generation.
    test_qvalue!(0, "0.0");
    test_qvalue!(1, "0.001");
    test_qvalue!(10, "0.01");
    test_qvalue!(999, "0.999");
    test_qvalue!(1000, "1.0");
  }
}
