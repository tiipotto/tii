//! Provides functionality for handling MIME types.

use std::fmt::Display;

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
  Other(String),
}

static WELL_KNOWN: &[MimeType] = &[
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
      MimeType::Other(_) => "bin",
    }
  }

  /// Does this mime type have an extension that is only used by this mime type and not shared with any other well known mime type?
  /// Types where this returns true cannot be relied upon to work with `MimeType::from_extension`
  pub const fn has_unique_known_extension(&self) -> bool {
    match self {
      MimeType::Video3gpp2 | MimeType::Audio3gpp2 => false, //3g2 is shared
      MimeType::Video3gpp | MimeType::Audio3gpp => false,   //3gp is shared
      MimeType::Other(_) => false, //We don't know what the extension even is.
      _ => true,
    }
  }

  /// returns a static slice that contains all well known mime types.
  #[must_use]
  pub fn well_known() -> &'static [MimeType] {
    WELL_KNOWN
  }

  /// returns true if this is a well known http method.
  #[must_use]
  pub const fn is_well_known(&self) -> bool {
    !matches!(self, MimeType::Other(_))
  }

  /// returns true if this is a custom http method.
  #[must_use]
  pub const fn is_custom(&self) -> bool {
    matches!(self, Self::Other(_))
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
      MimeType::Other(_) => return None,
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
      MimeType::Other(data) => data.as_str(),
    }
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

          if char <= 31 {
            //Ascii control characters, not allowed here!
            return None;
          }

          if char & 0b1000_0000 != 0 {
            //Multibyte utf-8 not permitted, this must be ascii!
            return None;
          }

          if char.is_ascii_uppercase() {
            // Upper case not permitted. (TODO is this correct? In practice ive only ever seen them lower case.)
            return None;
          }

          //TODO actually lookup the RFC and verify what exact printable characters are permitted here.
          if char == b'*' {
            // I know this one is not allowed.
            return None;
          }
        }

        if !found_slash {
          return None;
        }

        MimeType::Other(other.to_string())
      }
    })
  }
}

impl Display for MimeType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}
