//! Provides functionality for handling MIME types.

use crate::util::unwrap_some;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};

/// QValue is defined as a fixed point number with up to 3 digits
/// after comma. with a valid range from 0 to 1.
/// We represent this as an u16 from 0 to 1000.
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
#[repr(transparent)]
pub struct QValue(u16);

impl QValue {
  /// q=1.0
  pub const MAX: QValue = QValue(1000);

  /// q=0.0
  pub const MIN: QValue = QValue(0);

  /// Parses the QValue in http header representation.
  /// Note: this is without the "q=" prefix!
  /// Returns none if the value is either out of bounds or otherwise invalid.
  pub fn parse(qvalue: impl AsRef<str>) -> Option<QValue> {
    let qvalue = qvalue.as_ref();
    match qvalue.len() {
      1 => {
        if qvalue == "1" {
          return Some(QValue(1000));
        }
        if qvalue == "0" {
          return Some(QValue(0));
        }

        None
      }
      2 => None,
      3 => {
        if !qvalue.starts_with("0.") {
          if qvalue == "1.0" {
            return Some(QValue(1000));
          }
          return None;
        }

        if let Ok(value) = qvalue[2..].parse::<u16>() {
          return Some(QValue(value * 100));
        }

        None
      }
      4 => {
        if !qvalue.starts_with("0.") {
          if qvalue == "1.00" {
            return Some(QValue(1000));
          }
          return None;
        }

        if let Ok(value) = qvalue[2..].parse::<u16>() {
          return Some(QValue(value * 10));
        }

        None
      }
      5 => {
        if !qvalue.starts_with("0.") {
          if qvalue == "1.000" {
            return Some(QValue(1000));
          }
          return None;
        }

        if let Ok(value) = qvalue[2..].parse::<u16>() {
          return Some(QValue(value));
        }

        None
      }
      _ => None,
    }
  }

  /// Returns the QValue in http header representation.
  /// Note: this is without the "q=" prefix!
  pub const fn as_str(&self) -> &'static str {
    constutils::qvalue_to_strs!()
  }

  /// returns this QValue as an u16. This value always ranges from 0 to 1000.
  /// 1000 corresponds to 1.0 since q-values are fixed point numbers with up to 3 digits after comma.
  pub const fn as_u16(&self) -> u16 {
    self.0
  }

  /// Returns a QValue from the given u16. Parameters greater than 1000 are clamped to 1000.
  pub const fn from_clamped(qvalue: u16) -> QValue {
    if qvalue > 1000 {
      return QValue(1000);
    }

    QValue(qvalue)
  }
}

impl Display for QValue {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}
impl Default for QValue {
  fn default() -> Self {
    QValue(1000)
  }
}

/// Version of MimeType that can contain "*" symbols.
#[derive(Clone, PartialEq, Debug, Eq, Hash)]
pub enum AcceptMimeType {
  /// video/* or text/* or ...
  GroupWildcard(MimeGroup),
  /// text/html or application/json or ...
  Specific(MimeType),
  /// */*
  Wildcard,
}

impl AsRef<AcceptMimeType> for AcceptMimeType {
  fn as_ref(&self) -> &AcceptMimeType {
    self
  }
}

impl AcceptMimeType {
  /// Parses an accept mime type.
  pub fn parse(value: impl AsRef<str>) -> Option<AcceptMimeType> {
    let mime = value.as_ref();
    let mime = mime.split_once(";").map(|(mime, _)| mime).unwrap_or(mime);

    if mime == "*/*" {
      return Some(AcceptMimeType::Wildcard);
    }
    match MimeType::parse(mime) {
      None => match MimeGroup::parse(mime) {
        Some(group) => {
          if &mime[group.as_str().len()..] != "/*" {
            return None;
          }

          Some(AcceptMimeType::GroupWildcard(group))
        }
        None => None,
      },
      Some(mime) => Some(AcceptMimeType::Specific(mime)),
    }
  }

  /// Returns true if this AcceptMimeType permits the given mime type.
  pub fn permits_specific(&self, mime_type: impl AsRef<MimeType>) -> bool {
    match self {
      AcceptMimeType::GroupWildcard(group) => group == mime_type.as_ref().mime_group(),
      AcceptMimeType::Specific(mime) => mime == mime_type.as_ref(),
      AcceptMimeType::Wildcard => true,
    }
  }

  /// Returns true if this AcceptMimeType will accept ANY mime from the given group.
  pub fn permits_group(&self, mime_group: impl AsRef<MimeGroup>) -> bool {
    match self {
      AcceptMimeType::GroupWildcard(group) => group == mime_group.as_ref(),
      AcceptMimeType::Specific(_) => false,
      AcceptMimeType::Wildcard => true,
    }
  }

  /// Returns true if this AcceptMimeType will permit ANY mime type permitted by the other AcceptMimeType.
  pub fn permits(&self, mime_type: impl AsRef<AcceptMimeType>) -> bool {
    match mime_type.as_ref() {
      AcceptMimeType::GroupWildcard(group) => self.permits_group(group),
      AcceptMimeType::Specific(mime) => self.permits_specific(mime),
      AcceptMimeType::Wildcard => matches!(self, AcceptMimeType::Wildcard),
    }
  }
}

impl Display for AcceptMimeType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      AcceptMimeType::GroupWildcard(group) => {
        f.write_str(group.as_str())?;
        f.write_str("/*")?;
      }
      AcceptMimeType::Specific(mime) => {
        f.write_str(mime.as_str())?;
      }
      AcceptMimeType::Wildcard => f.write_str("*/*")?,
    }

    Ok(())
  }
}

///
/// Represents one part of an accept mime
/// # See
/// <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept>
#[derive(Clone, PartialEq, Debug, Eq)]
pub struct AcceptQualityMimeType {
  value: AcceptMimeType,
  q: QValue,
}

impl PartialOrd<Self> for AcceptQualityMimeType {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for AcceptQualityMimeType {
  fn cmp(&self, other: &Self) -> Ordering {
    other.q.cmp(&self.q)
  }
}

impl AcceptQualityMimeType {
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

        let qvalue = QValue::parse(&rawq[2..])?;

        if mime == "*/*" {
          data.push(AcceptQualityMimeType { value: AcceptMimeType::Wildcard, q: qvalue });
          continue;
        }

        match MimeType::parse(mime) {
          None => match MimeGroup::parse(mime) {
            Some(group) => {
              if &mime[group.as_str().len()..] != "/*" {
                return None;
              }
              data.push(AcceptQualityMimeType {
                value: AcceptMimeType::GroupWildcard(group),
                q: qvalue,
              })
            }
            None => return None,
          },
          Some(mime) => {
            data.push(AcceptQualityMimeType { value: AcceptMimeType::Specific(mime), q: qvalue })
          }
        };

        continue;
      }

      if mime == "*/*" {
        data.push(AcceptQualityMimeType { value: AcceptMimeType::Wildcard, q: QValue::default() });
        continue;
      }

      match MimeType::parse(mime) {
        None => match MimeGroup::parse(mime) {
          Some(group) => {
            if &mime[group.as_str().len()..] != "/*" {
              return None;
            }
            data.push(AcceptQualityMimeType {
              value: AcceptMimeType::GroupWildcard(group),
              q: QValue::default(),
            })
          }
          None => return None,
        },
        Some(mime) => data.push(AcceptQualityMimeType {
          value: AcceptMimeType::Specific(mime),
          q: QValue::default(),
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
  pub fn get_type(&self) -> &AcceptMimeType {
    &self.value
  }

  /// Get the QValue of this accept mime.
  pub const fn qvalue(&self) -> QValue {
    self.q
  }

  /// Is this a */* accept?
  pub const fn is_wildcard(&self) -> bool {
    matches!(self.value, AcceptMimeType::Wildcard)
  }

  /// Is this a group wildcard? i.e: `video/*` or `text/*`
  pub const fn is_group_wildcard(&self) -> bool {
    matches!(self.value, AcceptMimeType::GroupWildcard(_))
  }

  /// Is this a non wildcard mime? i.e: `video/mp4`
  pub const fn is_specific(&self) -> bool {
    matches!(self.value, AcceptMimeType::Specific(_))
  }

  /// Get the mime type. returns none if this is any type of wildcard mime
  pub const fn mime(&self) -> Option<&MimeType> {
    match &self.value {
      AcceptMimeType::Specific(mime) => Some(mime),
      _ => None,
    }
  }

  /// Get the mime type. returns none if this is the `*/*` mime.
  pub const fn group(&self) -> Option<&MimeGroup> {
    match &self.value {
      AcceptMimeType::Specific(mime) => Some(mime.mime_group()),
      AcceptMimeType::GroupWildcard(group) => Some(group),
      _ => None,
    }
  }

  /// Returns a AcceptMime equivalent to calling parse with `*/*`
  pub const fn wildcard(q: QValue) -> AcceptQualityMimeType {
    AcceptQualityMimeType { value: AcceptMimeType::Wildcard, q }
  }

  /// Returns a AcceptMime equivalent to calling parse with `group/*` depending on MimeGroup.
  pub const fn from_group(group: MimeGroup, q: QValue) -> AcceptQualityMimeType {
    AcceptQualityMimeType { value: AcceptMimeType::GroupWildcard(group), q }
  }

  /// Returns a AcceptMime equivalent to calling parse with `group/type` depending on MimeType.
  pub const fn from_mime(mime: MimeType, q: QValue) -> AcceptQualityMimeType {
    AcceptQualityMimeType { value: AcceptMimeType::Specific(mime), q }
  }
}

impl Default for AcceptQualityMimeType {
  fn default() -> Self {
    AcceptQualityMimeType::wildcard(QValue::default())
  }
}

impl Display for AcceptQualityMimeType {
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
pub enum MimeGroup {
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

impl AsRef<MimeGroup> for MimeGroup {
  fn as_ref(&self) -> &MimeGroup {
    self
  }
}

impl Display for MimeGroup {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

const WELL_KNOWN_GROUPS: &[MimeGroup] = &[
  MimeGroup::Font,
  MimeGroup::Application,
  MimeGroup::Image,
  MimeGroup::Video,
  MimeGroup::Audio,
  MimeGroup::Text,
];
impl MimeGroup {
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
      "font" => MimeGroup::Font,
      "application" => MimeGroup::Application,
      "image" => MimeGroup::Image,
      "video" => MimeGroup::Video,
      "audio" => MimeGroup::Audio,
      "text" => MimeGroup::Text,
      _ => MimeGroup::Other(value.to_string()),
    })
  }

  /// returns a static array over all well known mime groups.
  #[must_use]
  pub const fn well_known() -> &'static [MimeGroup] {
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
      MimeGroup::Font => "font",
      MimeGroup::Application => "application",
      MimeGroup::Image => "image",
      MimeGroup::Video => "video",
      MimeGroup::Audio => "audio",
      MimeGroup::Text => "text",
      MimeGroup::Other(_) => return None,
    })
  }

  /// returns the str name of the mime group.
  /// This name can be fed back into parse to get the equivalent enum of self.
  pub fn as_str(&self) -> &str {
    match self {
      MimeGroup::Font => "font",
      MimeGroup::Application => "application",
      MimeGroup::Image => "image",
      MimeGroup::Video => "video",
      MimeGroup::Audio => "audio",
      MimeGroup::Text => "text",
      MimeGroup::Other(o) => o.as_str(),
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
pub enum MimeType {
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
  Other(MimeGroup, String),
}

impl AsRef<MimeType> for MimeType {
  fn as_ref(&self) -> &MimeType {
    self
  }
}

const WELL_KNOWN_TYPES: &[MimeType] = &[
  MimeType::FontTtf,
  MimeType::FontOtf,
  MimeType::FontWoff,
  MimeType::FontWoff2,
  MimeType::ApplicationAbiWord,
  MimeType::ApplicationFreeArc,
  MimeType::ApplicationAmazonEbook,
  MimeType::ApplicationBzip,
  MimeType::ApplicationBzip2,
  MimeType::ApplicationCDAudio,
  MimeType::ApplicationCShell,
  MimeType::ApplicationMicrosoftWord,
  MimeType::ApplicationMicrosoftWordXml,
  MimeType::ApplicationMicrosoftFont,
  MimeType::ApplicationEpub,
  MimeType::ApplicationGzip,
  MimeType::ApplicationJar,
  MimeType::ApplicationJavaClass,
  MimeType::ApplicationOctetStream,
  MimeType::ApplicationJson,
  MimeType::ApplicationJsonLd,
  MimeType::ApplicationPdf,
  MimeType::ApplicationZip,
  MimeType::ApplicationAppleInstallerPackage,
  MimeType::ApplicationOpenDocumentPresentation,
  MimeType::ApplicationOpenDocumentSpreadsheet,
  MimeType::ApplicationOpenDocumentText,
  MimeType::ApplicationOgg,
  MimeType::ApplicationPhp,
  MimeType::ApplicationMicrosoftPowerpoint,
  MimeType::ApplicationMicrosoftPowerpointXml,
  MimeType::ApplicationRar,
  MimeType::ApplicationRichText,
  MimeType::ApplicationBourneShell,
  MimeType::ApplicationTapeArchive,
  MimeType::ApplicationMicrosoftVisio,
  MimeType::ApplicationXHtml,
  MimeType::ApplicationMicrosoftExcel,
  MimeType::ApplicationMicrosoftExcelXml,
  MimeType::ApplicationXml,
  MimeType::ApplicationXul,
  MimeType::ApplicationDicom,
  MimeType::Application7Zip,
  MimeType::ApplicationWasm,
  MimeType::VideoMp4,
  MimeType::VideoOgg,
  MimeType::VideoWebm,
  MimeType::VideoAvi,
  MimeType::VideoMpeg,
  MimeType::VideoMpegTransportStream,
  MimeType::Video3gpp,
  MimeType::Video3gpp2,
  MimeType::ImageBmp,
  MimeType::ImageGif,
  MimeType::ImageJpeg,
  MimeType::ImageAvif,
  MimeType::ImagePng,
  MimeType::ImageApng,
  MimeType::ImageWebp,
  MimeType::ImageSvg,
  MimeType::ImageIcon,
  MimeType::ImageTiff,
  MimeType::AudioAac,
  MimeType::AudioMidi,
  MimeType::AudioMpeg,
  MimeType::AudioOgg,
  MimeType::AudioWaveform,
  MimeType::AudioWebm,
  MimeType::Audio3gpp,
  MimeType::Audio3gpp2,
  MimeType::TextCss,
  MimeType::TextHtml,
  MimeType::TextJavaScript,
  MimeType::TextPlain,
  MimeType::TextCsv,
  MimeType::TextCalendar,
  MimeType::ApplicationYaml,
  MimeType::TextLua,
  MimeType::ApplicationLuaBytecode,
  MimeType::ApplicationXz,
];

impl MimeType {
  /// Converts from a file extension without the `.` to the enum variant.
  /// If the MIME type cannot be inferred from the extension, returns `MimeType::ApplicationOctetStream`.
  pub fn from_extension(extension: impl AsRef<str>) -> Self {
    //TODO Heap allocation to_ascii_lowercase
    match extension.as_ref().to_ascii_lowercase().as_str() {
      "css" => MimeType::TextCss,
      "html" => MimeType::TextHtml,
      "htm" => MimeType::TextHtml,
      "js" => MimeType::TextJavaScript,
      "mjs" => MimeType::TextJavaScript,
      "txt" => MimeType::TextPlain,
      "bmp" => MimeType::ImageBmp,
      "gif" => MimeType::ImageGif,
      "jpeg" => MimeType::ImageJpeg,
      "jpg" => MimeType::ImageJpeg,
      "png" => MimeType::ImagePng,
      "webp" => MimeType::ImageWebp,
      "svg" => MimeType::ImageSvg,
      "ico" => MimeType::ImageIcon,
      "json" => MimeType::ApplicationJson,
      "pdf" => MimeType::ApplicationPdf,
      "zip" => MimeType::ApplicationZip,
      "mp4" => MimeType::VideoMp4,
      "ogv" => MimeType::VideoOgg,
      "webm" => MimeType::VideoWebm,
      "ttf" => MimeType::FontTtf,
      "otf" => MimeType::FontOtf,
      "woff" => MimeType::FontWoff,
      "woff2" => MimeType::FontWoff2,
      "abw" => MimeType::ApplicationAbiWord,
      "arc" => MimeType::ApplicationFreeArc,
      "azw" => MimeType::ApplicationAmazonEbook,
      "bz" => MimeType::ApplicationBzip,
      "bz2" => MimeType::ApplicationBzip2,
      "cda" => MimeType::ApplicationCDAudio,
      "csh" => MimeType::ApplicationCShell,
      "doc" => MimeType::ApplicationMicrosoftWord,
      "docx" => MimeType::ApplicationMicrosoftWordXml,
      "eot" => MimeType::ApplicationMicrosoftFont,
      "epub" => MimeType::ApplicationEpub,
      "gz" => MimeType::ApplicationGzip,
      "jar" => MimeType::ApplicationJar,
      "class" => MimeType::ApplicationJavaClass,
      "bin" => MimeType::ApplicationOctetStream,
      "jsonld" => MimeType::ApplicationJsonLd,
      "mpkg" => MimeType::ApplicationAppleInstallerPackage,
      "odp" => MimeType::ApplicationOpenDocumentPresentation,
      "ods" => MimeType::ApplicationOpenDocumentSpreadsheet,
      "odt" => MimeType::ApplicationOpenDocumentText,
      "ogx" => MimeType::ApplicationOgg,
      "php" => MimeType::ApplicationPhp,
      "ppt" => MimeType::ApplicationMicrosoftPowerpoint,
      "pptx" => MimeType::ApplicationMicrosoftPowerpointXml,
      "rar" => MimeType::ApplicationRar,
      "rtf" => MimeType::ApplicationRichText,
      "sh" => MimeType::ApplicationBourneShell,
      "tar" => MimeType::ApplicationTapeArchive,
      "vsd" => MimeType::ApplicationMicrosoftVisio,
      "xhtml" => MimeType::ApplicationXHtml,
      "xls" => MimeType::ApplicationMicrosoftExcel,
      "xlsx" => MimeType::ApplicationMicrosoftExcelXml,
      "xml" => MimeType::ApplicationXml,
      "xul" => MimeType::ApplicationXul,
      "dcm" => MimeType::ApplicationDicom,
      "7z" => MimeType::Application7Zip,
      "wasm" => MimeType::ApplicationWasm,
      "avi" => MimeType::VideoAvi,
      "mpeg" => MimeType::VideoMpeg,
      "ts" => MimeType::VideoMpegTransportStream,
      "3gp" => MimeType::Video3gpp,
      "3g2" => MimeType::Video3gpp2,
      "avif" => MimeType::ImageAvif,
      "apng" => MimeType::ImageApng,
      "tif" => MimeType::ImageTiff,
      "aac" => MimeType::AudioAac,
      "mid" => MimeType::AudioMidi,
      "mp3" => MimeType::AudioMpeg,
      "oga" => MimeType::AudioOgg,
      "wav" => MimeType::AudioWaveform,
      "weba" => MimeType::AudioWebm,
      "csv" => MimeType::TextCsv,
      "cal" => MimeType::TextCalendar,
      "yaml" | "yml" => MimeType::ApplicationYaml,
      "lua" => MimeType::TextLua,
      "luac" => MimeType::ApplicationLuaBytecode,
      "xz" => MimeType::ApplicationXz,
      _ => MimeType::ApplicationOctetStream,
    }
  }

  /// returns the file extension that is most likely correct for the given file type.
  /// For mime types where this is not clear "bin" is returned.
  #[must_use]
  pub const fn extension(&self) -> &'static str {
    match self {
      MimeType::FontTtf => "ttf",
      MimeType::FontOtf => "otf",
      MimeType::FontWoff => "woff",
      MimeType::FontWoff2 => "woff2",
      MimeType::ApplicationAbiWord => "abw",
      MimeType::ApplicationFreeArc => "arc",
      MimeType::ApplicationAmazonEbook => "azw",
      MimeType::ApplicationBzip => "bz",
      MimeType::ApplicationBzip2 => "bz2",
      MimeType::ApplicationCDAudio => "cda",
      MimeType::ApplicationCShell => "csh",
      MimeType::ApplicationMicrosoftWord => "doc",
      MimeType::ApplicationMicrosoftWordXml => "docx",
      MimeType::ApplicationMicrosoftFont => "eot",
      MimeType::ApplicationEpub => "epub",
      MimeType::ApplicationGzip => "gz",
      MimeType::ApplicationJar => "jar",
      MimeType::ApplicationJavaClass => "class",
      MimeType::ApplicationOctetStream => "bin",
      MimeType::ApplicationJson => "json",
      MimeType::ApplicationJsonLd => "jsonld",
      MimeType::ApplicationPdf => "pdf",
      MimeType::ApplicationZip => "zip",
      MimeType::ApplicationAppleInstallerPackage => "mpkg",
      MimeType::ApplicationOpenDocumentPresentation => "odp",
      MimeType::ApplicationOpenDocumentSpreadsheet => "ods",
      MimeType::ApplicationOpenDocumentText => "odt",
      MimeType::ApplicationOgg => "ogx",
      MimeType::ApplicationPhp => "php",
      MimeType::ApplicationMicrosoftPowerpoint => "ppt",
      MimeType::ApplicationMicrosoftPowerpointXml => "pptx",
      MimeType::ApplicationRar => "rar",
      MimeType::ApplicationRichText => "rtf",
      MimeType::ApplicationBourneShell => "sh",
      MimeType::ApplicationTapeArchive => "tar",
      MimeType::ApplicationMicrosoftVisio => "vsd",
      MimeType::ApplicationXHtml => "xhtml",
      MimeType::ApplicationMicrosoftExcel => "xls",
      MimeType::ApplicationMicrosoftExcelXml => "xlsx",
      MimeType::ApplicationXml => "xml",
      MimeType::ApplicationXul => "xul",
      MimeType::ApplicationDicom => "dcm",
      MimeType::Application7Zip => "7z",
      MimeType::ApplicationWasm => "wasm",
      MimeType::VideoMp4 => "mp4",
      MimeType::VideoOgg => "ogv",
      MimeType::VideoWebm => "webm",
      MimeType::VideoAvi => "avi",
      MimeType::VideoMpeg => "mpeg",
      MimeType::VideoMpegTransportStream => "ts",
      MimeType::Video3gpp => "3gp",
      MimeType::Video3gpp2 => "3g2",
      MimeType::ImageBmp => "bmp",
      MimeType::ImageGif => "gif",
      MimeType::ImageJpeg => "jpg",
      MimeType::ImageAvif => "avif",
      MimeType::ImagePng => "png",
      MimeType::ImageApng => "apng",
      MimeType::ImageWebp => "webp",
      MimeType::ImageSvg => "svg",
      MimeType::ImageIcon => "ico",
      MimeType::ImageTiff => "tif",
      MimeType::AudioAac => "aac",
      MimeType::AudioMidi => "mid",
      MimeType::AudioMpeg => "mp3",
      MimeType::AudioOgg => "oga",
      MimeType::AudioWaveform => "wav",
      MimeType::AudioWebm => "weba",
      MimeType::Audio3gpp => "3gp",
      MimeType::Audio3gpp2 => "3g2",
      MimeType::TextCss => "css",
      MimeType::TextHtml => "html",
      MimeType::TextJavaScript => "js",
      MimeType::TextPlain => "txt",
      MimeType::TextCsv => "csv",
      MimeType::TextCalendar => "cal",
      MimeType::ApplicationYaml => "yaml",
      MimeType::TextLua => "lua",
      MimeType::ApplicationLuaBytecode => "luac",
      MimeType::ApplicationXz => "xz",
      MimeType::Other(_, _) => "bin",
    }
  }

  /// returns the MimeGroup of this mime type.
  pub const fn mime_group(&self) -> &MimeGroup {
    match self {
      MimeType::FontTtf => &MimeGroup::Font,
      MimeType::FontOtf => &MimeGroup::Font,
      MimeType::FontWoff => &MimeGroup::Font,
      MimeType::FontWoff2 => &MimeGroup::Font,
      MimeType::ApplicationAbiWord => &MimeGroup::Application,
      MimeType::ApplicationFreeArc => &MimeGroup::Application,
      MimeType::ApplicationAmazonEbook => &MimeGroup::Application,
      MimeType::ApplicationBzip => &MimeGroup::Application,
      MimeType::ApplicationBzip2 => &MimeGroup::Application,
      MimeType::ApplicationCDAudio => &MimeGroup::Application,
      MimeType::ApplicationCShell => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftWord => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftWordXml => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftFont => &MimeGroup::Application,
      MimeType::ApplicationEpub => &MimeGroup::Application,
      MimeType::ApplicationGzip => &MimeGroup::Application,
      MimeType::ApplicationJar => &MimeGroup::Application,
      MimeType::ApplicationJavaClass => &MimeGroup::Application,
      MimeType::ApplicationOctetStream => &MimeGroup::Application,
      MimeType::ApplicationJson => &MimeGroup::Application,
      MimeType::ApplicationJsonLd => &MimeGroup::Application,
      MimeType::ApplicationYaml => &MimeGroup::Application,
      MimeType::ApplicationLuaBytecode => &MimeGroup::Application,
      MimeType::ApplicationPdf => &MimeGroup::Application,
      MimeType::ApplicationZip => &MimeGroup::Application,
      MimeType::ApplicationAppleInstallerPackage => &MimeGroup::Application,
      MimeType::ApplicationOpenDocumentPresentation => &MimeGroup::Application,
      MimeType::ApplicationOpenDocumentSpreadsheet => &MimeGroup::Application,
      MimeType::ApplicationOpenDocumentText => &MimeGroup::Application,
      MimeType::ApplicationOgg => &MimeGroup::Application,
      MimeType::ApplicationPhp => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftPowerpoint => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftPowerpointXml => &MimeGroup::Application,
      MimeType::ApplicationRar => &MimeGroup::Application,
      MimeType::ApplicationRichText => &MimeGroup::Application,
      MimeType::ApplicationBourneShell => &MimeGroup::Application,
      MimeType::ApplicationTapeArchive => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftVisio => &MimeGroup::Application,
      MimeType::ApplicationXHtml => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftExcel => &MimeGroup::Application,
      MimeType::ApplicationMicrosoftExcelXml => &MimeGroup::Application,
      MimeType::ApplicationXml => &MimeGroup::Application,
      MimeType::ApplicationXul => &MimeGroup::Application,
      MimeType::ApplicationDicom => &MimeGroup::Application,
      MimeType::Application7Zip => &MimeGroup::Application,
      MimeType::ApplicationXz => &MimeGroup::Application,
      MimeType::ApplicationWasm => &MimeGroup::Application,
      MimeType::VideoMp4 => &MimeGroup::Video,
      MimeType::VideoOgg => &MimeGroup::Video,
      MimeType::VideoWebm => &MimeGroup::Video,
      MimeType::VideoAvi => &MimeGroup::Video,
      MimeType::VideoMpeg => &MimeGroup::Video,
      MimeType::VideoMpegTransportStream => &MimeGroup::Video,
      MimeType::Video3gpp => &MimeGroup::Video,
      MimeType::Video3gpp2 => &MimeGroup::Video,
      MimeType::ImageBmp => &MimeGroup::Image,
      MimeType::ImageGif => &MimeGroup::Image,
      MimeType::ImageJpeg => &MimeGroup::Image,
      MimeType::ImageAvif => &MimeGroup::Image,
      MimeType::ImagePng => &MimeGroup::Image,
      MimeType::ImageApng => &MimeGroup::Image,
      MimeType::ImageWebp => &MimeGroup::Image,
      MimeType::ImageSvg => &MimeGroup::Image,
      MimeType::ImageIcon => &MimeGroup::Image,
      MimeType::ImageTiff => &MimeGroup::Image,
      MimeType::AudioAac => &MimeGroup::Audio,
      MimeType::AudioMidi => &MimeGroup::Audio,
      MimeType::AudioMpeg => &MimeGroup::Audio,
      MimeType::AudioOgg => &MimeGroup::Audio,
      MimeType::AudioWaveform => &MimeGroup::Audio,
      MimeType::AudioWebm => &MimeGroup::Audio,
      MimeType::Audio3gpp => &MimeGroup::Audio,
      MimeType::Audio3gpp2 => &MimeGroup::Audio,
      MimeType::TextCss => &MimeGroup::Text,
      MimeType::TextHtml => &MimeGroup::Text,
      MimeType::TextJavaScript => &MimeGroup::Text,
      MimeType::TextLua => &MimeGroup::Text,
      MimeType::TextPlain => &MimeGroup::Text,
      MimeType::TextCsv => &MimeGroup::Text,
      MimeType::TextCalendar => &MimeGroup::Text,
      MimeType::Other(group, _) => group,
    }
  }

  /// Does this mime type have an extension that is only used by this mime type and not shared with any other well known mime type?
  /// Types where this returns true cannot be relied upon to work with `MimeType::from_extension`
  pub const fn has_unique_known_extension(&self) -> bool {
    match self {
      MimeType::Video3gpp2 | MimeType::Audio3gpp2 => false, //3g2 is shared
      MimeType::Video3gpp | MimeType::Audio3gpp => false,   //3gp is shared
      MimeType::Other(_, _) => false, //We don't know what the extension even is.
      _ => true,
    }
  }

  /// returns a static slice that contains all well known mime types.
  #[must_use]
  pub const fn well_known() -> &'static [MimeType] {
    WELL_KNOWN_TYPES
  }

  /// returns true if this is a well known http mime type.
  #[must_use]
  pub const fn is_well_known(&self) -> bool {
    !matches!(self, MimeType::Other(_, _))
  }

  /// returns true if this is a custom http mime type.
  #[must_use]
  pub const fn is_custom(&self) -> bool {
    matches!(self, Self::Other(_, _))
  }

  /// Returns a static str of the mime type or None if the mime type is heap allocated.
  pub const fn well_known_str(&self) -> Option<&'static str> {
    Some(match self {
      MimeType::TextCss => "text/css",
      MimeType::TextHtml => "text/html",
      MimeType::TextJavaScript => "text/javascript",
      MimeType::TextPlain => "text/plain",
      MimeType::ImageBmp => "image/bmp",
      MimeType::ImageGif => "image/gif",
      MimeType::ImageJpeg => "image/jpeg",
      MimeType::ImagePng => "image/png",
      MimeType::ImageWebp => "image/webp",
      MimeType::ImageSvg => "image/svg+xml",
      MimeType::ImageIcon => "image/vnd.microsoft.icon",
      MimeType::ApplicationOctetStream => "application/octet-stream",
      MimeType::ApplicationJson => "application/json",
      MimeType::ApplicationPdf => "application/pdf",
      MimeType::ApplicationZip => "application/zip",
      MimeType::VideoMp4 => "video/mp4",
      MimeType::VideoOgg => "video/ogg",
      MimeType::VideoWebm => "video/webm",
      MimeType::FontTtf => "font/ttf",
      MimeType::FontOtf => "font/otf",
      MimeType::FontWoff => "font/woff",
      MimeType::FontWoff2 => "font/woff2",
      MimeType::ApplicationAbiWord => "application/x-abiword",
      MimeType::ApplicationFreeArc => "application/x-freearc",
      MimeType::ApplicationAmazonEbook => "application/vnd.amazon.ebook",
      MimeType::ApplicationBzip => "application/x-bzip",
      MimeType::ApplicationBzip2 => "application/x-bzip2",
      MimeType::ApplicationCDAudio => "application/x-cdf",
      MimeType::ApplicationCShell => "application/x-csh",
      MimeType::ApplicationMicrosoftWord => "application/msword",
      MimeType::ApplicationMicrosoftWordXml => {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
      }
      MimeType::ApplicationMicrosoftFont => "application/vnd.ms-fontobject",
      MimeType::ApplicationEpub => "application/epub+zip",
      MimeType::ApplicationGzip => "application/gzip",
      MimeType::ApplicationJar => "application/java-archive",
      MimeType::ApplicationJavaClass => "application/x-java-class",
      MimeType::ApplicationJsonLd => "application/ld+json",
      MimeType::ApplicationAppleInstallerPackage => "application/vnd.apple.installer+xml",
      MimeType::ApplicationOpenDocumentPresentation => {
        "application/vnd.oasis.opendocument.presentation"
      }
      MimeType::ApplicationOpenDocumentSpreadsheet => {
        "application/vnd.oasis.opendocument.spreadsheet"
      }
      MimeType::ApplicationOpenDocumentText => "application/vnd.oasis.opendocument.text",
      MimeType::ApplicationOgg => "application/ogg",
      MimeType::ApplicationPhp => "application/x-httpd-php",
      MimeType::ApplicationMicrosoftPowerpoint => "application/vnd.ms-powerpoint",
      MimeType::ApplicationMicrosoftPowerpointXml => {
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
      }
      MimeType::ApplicationRar => "application/vnd.rar",
      MimeType::ApplicationRichText => "application/rtf",
      MimeType::ApplicationBourneShell => "application/x-sh",
      MimeType::ApplicationTapeArchive => "application/x-tar",
      MimeType::ApplicationMicrosoftVisio => "application/vnd.visio",
      MimeType::ApplicationXHtml => "application/xhtml+xml",
      MimeType::ApplicationMicrosoftExcel => "application/vnd.ms-excel",
      MimeType::ApplicationMicrosoftExcelXml => {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      }
      MimeType::ApplicationXml => "application/xml",
      MimeType::ApplicationXul => "application/vnd.mozilla.xul+xml",
      MimeType::ApplicationDicom => "application/dicom",
      MimeType::Application7Zip => "application/x-7z-compressed",
      MimeType::ApplicationWasm => "application/wasm",
      MimeType::VideoAvi => "video/x-msvideo",
      MimeType::VideoMpeg => "video/mpeg",
      MimeType::VideoMpegTransportStream => "video/mp2t",
      MimeType::Video3gpp => "video/3gpp",
      MimeType::Video3gpp2 => "video/3gpp2",
      MimeType::ImageAvif => "image/avif",
      MimeType::ImageApng => "image/apng",
      MimeType::ImageTiff => "image/tiff",
      MimeType::AudioAac => "audio/aac",
      MimeType::AudioMidi => "audio/midi",
      MimeType::AudioMpeg => "audio/mpeg",
      MimeType::AudioOgg => "audio/ogg",
      MimeType::AudioWaveform => "audio/wav",
      MimeType::AudioWebm => "audio/webm",
      MimeType::Audio3gpp => "audio/3gpp",
      MimeType::Audio3gpp2 => "audio/3gpp2",
      MimeType::TextCsv => "text/csv",
      MimeType::TextCalendar => "text/calendar",
      MimeType::ApplicationYaml => "application/yaml",
      MimeType::TextLua => "text/x-lua",
      MimeType::ApplicationLuaBytecode => "application/x-lua-bytecode",
      MimeType::ApplicationXz => "application/x-xz",
      MimeType::Other(_, _) => return None,
    })
  }

  /// returns the &str representation of the mime type.
  pub fn as_str(&self) -> &str {
    match self {
      MimeType::TextCss => "text/css",
      MimeType::TextHtml => "text/html",
      MimeType::TextJavaScript => "text/javascript",
      MimeType::TextPlain => "text/plain",
      MimeType::ImageBmp => "image/bmp",
      MimeType::ImageGif => "image/gif",
      MimeType::ImageJpeg => "image/jpeg",
      MimeType::ImagePng => "image/png",
      MimeType::ImageWebp => "image/webp",
      MimeType::ImageSvg => "image/svg+xml",
      MimeType::ImageIcon => "image/vnd.microsoft.icon",
      MimeType::ApplicationOctetStream => "application/octet-stream",
      MimeType::ApplicationJson => "application/json",
      MimeType::ApplicationPdf => "application/pdf",
      MimeType::ApplicationZip => "application/zip",
      MimeType::VideoMp4 => "video/mp4",
      MimeType::VideoOgg => "video/ogg",
      MimeType::VideoWebm => "video/webm",
      MimeType::FontTtf => "font/ttf",
      MimeType::FontOtf => "font/otf",
      MimeType::FontWoff => "font/woff",
      MimeType::FontWoff2 => "font/woff2",
      MimeType::ApplicationAbiWord => "application/x-abiword",
      MimeType::ApplicationFreeArc => "application/x-freearc",
      MimeType::ApplicationAmazonEbook => "application/vnd.amazon.ebook",
      MimeType::ApplicationBzip => "application/x-bzip",
      MimeType::ApplicationBzip2 => "application/x-bzip2",
      MimeType::ApplicationCDAudio => "application/x-cdf",
      MimeType::ApplicationCShell => "application/x-csh",
      MimeType::ApplicationMicrosoftWord => "application/msword",
      MimeType::ApplicationMicrosoftWordXml => {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
      }
      MimeType::ApplicationMicrosoftFont => "application/vnd.ms-fontobject",
      MimeType::ApplicationEpub => "application/epub+zip",
      MimeType::ApplicationGzip => "application/gzip",
      MimeType::ApplicationJar => "application/java-archive",
      MimeType::ApplicationJavaClass => "application/x-java-class",
      MimeType::ApplicationJsonLd => "application/ld+json",
      MimeType::ApplicationAppleInstallerPackage => "application/vnd.apple.installer+xml",
      MimeType::ApplicationOpenDocumentPresentation => {
        "application/vnd.oasis.opendocument.presentation"
      }
      MimeType::ApplicationOpenDocumentSpreadsheet => {
        "application/vnd.oasis.opendocument.spreadsheet"
      }
      MimeType::ApplicationOpenDocumentText => "application/vnd.oasis.opendocument.text",
      MimeType::ApplicationOgg => "application/ogg",
      MimeType::ApplicationPhp => "application/x-httpd-php",
      MimeType::ApplicationMicrosoftPowerpoint => "application/vnd.ms-powerpoint",
      MimeType::ApplicationMicrosoftPowerpointXml => {
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
      }
      MimeType::ApplicationRar => "application/vnd.rar",
      MimeType::ApplicationRichText => "application/rtf",
      MimeType::ApplicationBourneShell => "application/x-sh",
      MimeType::ApplicationTapeArchive => "application/x-tar",
      MimeType::ApplicationMicrosoftVisio => "application/vnd.visio",
      MimeType::ApplicationXHtml => "application/xhtml+xml",
      MimeType::ApplicationMicrosoftExcel => "application/vnd.ms-excel",
      MimeType::ApplicationMicrosoftExcelXml => {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      }
      MimeType::ApplicationXml => "application/xml",
      MimeType::ApplicationXul => "application/vnd.mozilla.xul+xml",
      MimeType::ApplicationDicom => "application/dicom",
      MimeType::Application7Zip => "application/x-7z-compressed",
      MimeType::ApplicationWasm => "application/wasm",
      MimeType::VideoAvi => "video/x-msvideo",
      MimeType::VideoMpeg => "video/mpeg",
      MimeType::VideoMpegTransportStream => "video/mp2t",
      MimeType::Video3gpp => "video/3gpp",
      MimeType::Video3gpp2 => "video/3gpp2",
      MimeType::ImageAvif => "image/avif",
      MimeType::ImageApng => "image/apng",
      MimeType::ImageTiff => "image/tiff",
      MimeType::AudioAac => "audio/aac",
      MimeType::AudioMidi => "audio/midi",
      MimeType::AudioMpeg => "audio/mpeg",
      MimeType::AudioOgg => "audio/ogg",
      MimeType::AudioWaveform => "audio/wav",
      MimeType::AudioWebm => "audio/webm",
      MimeType::Audio3gpp => "audio/3gpp",
      MimeType::Audio3gpp2 => "audio/3gpp2",
      MimeType::TextCsv => "text/csv",
      MimeType::TextCalendar => "text/calendar",
      MimeType::ApplicationYaml => "application/yaml",
      MimeType::TextLua => "text/x-lua",
      MimeType::ApplicationLuaBytecode => "application/x-lua-bytecode",
      MimeType::ApplicationXz => "application/x-xz",
      MimeType::Other(_, data) => data.as_str(),
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
      "text/css" => MimeType::TextCss,
      "text/html" => MimeType::TextHtml,
      "text/javascript" => MimeType::TextJavaScript,
      "text/plain" => MimeType::TextPlain,
      "image/bmp" => MimeType::ImageBmp,
      "image/gif" => MimeType::ImageGif,
      "image/jpeg" => MimeType::ImageJpeg,
      "image/png" => MimeType::ImagePng,
      "image/webp" => MimeType::ImageWebp,
      "image/svg+xml" => MimeType::ImageSvg,
      "image/vnd.microsoft.icon" => MimeType::ImageIcon,
      "application/octet-stream" => MimeType::ApplicationOctetStream,
      "application/json" => MimeType::ApplicationJson,
      "application/pdf" => MimeType::ApplicationPdf,
      "application/zip" => MimeType::ApplicationZip,
      "video/mp4" => MimeType::VideoMp4,
      "video/ogg" => MimeType::VideoOgg,
      "video/webm" => MimeType::VideoWebm,
      "font/ttf" => MimeType::FontTtf,
      "font/otf" => MimeType::FontOtf,
      "font/woff" => MimeType::FontWoff,
      "font/woff2" => MimeType::FontWoff2,
      "application/x-abiword" => MimeType::ApplicationAbiWord,
      "application/x-freearc" => MimeType::ApplicationFreeArc,
      "application/vnd.amazon.ebook" => MimeType::ApplicationAmazonEbook,
      "application/x-bzip" => MimeType::ApplicationBzip,
      "application/x-bzip2" => MimeType::ApplicationBzip2,
      "application/x-cdf" => MimeType::ApplicationCDAudio,
      "application/x-csh" => MimeType::ApplicationCShell,
      "application/msword" => MimeType::ApplicationMicrosoftWord,
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
        MimeType::ApplicationMicrosoftWordXml
      }
      "application/vnd.ms-fontobject" => MimeType::ApplicationMicrosoftFont,
      "application/epub+zip" => MimeType::ApplicationEpub,
      "application/gzip" => MimeType::ApplicationGzip,
      "application/java-archive" => MimeType::ApplicationJar,
      "application/x-java-class" => MimeType::ApplicationJavaClass,
      "application/ld+json" => MimeType::ApplicationJsonLd,
      "application/vnd.apple.installer+xml" => MimeType::ApplicationAppleInstallerPackage,
      "application/vnd.oasis.opendocument.presentation" => {
        MimeType::ApplicationOpenDocumentPresentation
      }
      "application/vnd.oasis.opendocument.spreadsheet" => {
        MimeType::ApplicationOpenDocumentSpreadsheet
      }
      "application/vnd.oasis.opendocument.text" => MimeType::ApplicationOpenDocumentText,
      "application/ogg" => MimeType::ApplicationOgg,
      "application/x-httpd-php" => MimeType::ApplicationPhp,
      "application/vnd.ms-powerpoint" => MimeType::ApplicationMicrosoftPowerpoint,
      "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
        MimeType::ApplicationMicrosoftPowerpointXml
      }
      "application/vnd.rar" => MimeType::ApplicationRar,
      "application/rtf" => MimeType::ApplicationRichText,
      "application/x-sh" => MimeType::ApplicationBourneShell,
      "application/x-tar" => MimeType::ApplicationTapeArchive,
      "application/vnd.visio" => MimeType::ApplicationMicrosoftVisio,
      "application/xhtml+xml" => MimeType::ApplicationXHtml,
      "application/vnd.ms-excel" => MimeType::ApplicationMicrosoftExcel,
      "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
        MimeType::ApplicationMicrosoftExcelXml
      }
      "application/xml" => MimeType::ApplicationXml,
      "application/vnd.mozilla.xul+xml" => MimeType::ApplicationXul,
      "application/dicom" => MimeType::ApplicationDicom,
      "application/x-7z-compressed" => MimeType::Application7Zip,
      "application/wasm" => MimeType::ApplicationWasm,
      "video/x-msvideo" => MimeType::VideoAvi,
      "video/mpeg" => MimeType::VideoMpeg,
      "video/mp2t" => MimeType::VideoMpegTransportStream,
      "video/3gpp" => MimeType::Video3gpp,
      "video/3gpp2" => MimeType::Video3gpp2,
      "audio/3gpp" => MimeType::Audio3gpp,
      "audio/3gpp2" => MimeType::Audio3gpp2,
      "image/avif" => MimeType::ImageAvif,
      "image/apng" => MimeType::ImageApng,
      "image/tiff" => MimeType::ImageTiff,
      "audio/aac" => MimeType::AudioAac,
      "audio/midi" => MimeType::AudioMidi,
      "audio/mpeg" => MimeType::AudioMpeg,
      "audio/ogg" => MimeType::AudioOgg,
      "audio/wav" => MimeType::AudioWaveform,
      "audio/webm" => MimeType::AudioWebm,
      "text/csv" => MimeType::TextCsv,
      "text/calendar" => MimeType::TextCalendar,
      "application/yaml" => MimeType::ApplicationYaml,
      "text/x-lua" => MimeType::TextLua,
      "application/x-lua-bytecode" => MimeType::ApplicationLuaBytecode,
      "application/x-xz" => MimeType::ApplicationXz,
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

        if let Some(grp) = MimeGroup::parse(other) {
          MimeType::Other(grp, other.to_string())
        } else {
          // We already do a superset of validations, this case is impossible.
          crate::util::unreachable()
        }
      }
    })
  }
}

impl Display for MimeType {
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

impl From<MimeType> for AcceptMimeType {
  fn from(value: MimeType) -> Self {
    AcceptMimeType::Specific(value)
  }
}

impl From<&MimeType> for AcceptMimeType {
  fn from(value: &MimeType) -> Self {
    AcceptMimeType::Specific(value.clone())
  }
}

impl From<MimeGroup> for AcceptMimeType {
  fn from(value: MimeGroup) -> Self {
    AcceptMimeType::GroupWildcard(value)
  }
}

impl From<&MimeGroup> for AcceptMimeType {
  fn from(value: &MimeGroup) -> Self {
    AcceptMimeType::GroupWildcard(value.clone())
  }
}

impl From<MimeType> for MimeGroup {
  fn from(value: MimeType) -> Self {
    value.mime_group().clone()
  }
}

impl From<&MimeType> for MimeGroup {
  fn from(value: &MimeType) -> Self {
    value.mime_group().clone()
  }
}

impl From<AcceptQualityMimeType> for AcceptMimeType {
  fn from(value: AcceptQualityMimeType) -> Self {
    value.value
  }
}

#[cfg(test)]
mod tests {
  use crate::http::mime::QValue;

  /// Shutup clippy.
  #[macro_export]
  macro_rules! test_qvalue {
    ($input:expr, $expected:expr) => {
      let q = QValue($input);
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
