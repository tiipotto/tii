//! Provides functionality for handling MIME types.

use either::{Either, Left, Right};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

/// QValue is defined as a fixed point number with up to 3 digits
/// after comma. with a valid range from 0 to 1.
/// We represent this as an u16 from 0 to 1000.
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct QValue(u16);

impl QValue {
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
  #[allow(clippy::zero_prefixed_literal)]
  pub const fn as_str(&self) -> &'static str {
    // Yes I know this is long
    // I didn't find another way to make this const and return static str.

    match self.0 {
      0000 => "0.0",
      0001 => "0.001",
      0002 => "0.002",
      0003 => "0.003",
      0004 => "0.004",
      0005 => "0.005",
      0006 => "0.006",
      0007 => "0.007",
      0008 => "0.008",
      0009 => "0.009",
      0010 => "0.01",
      0011 => "0.011",
      0012 => "0.012",
      0013 => "0.013",
      0014 => "0.014",
      0015 => "0.015",
      0016 => "0.016",
      0017 => "0.017",
      0018 => "0.018",
      0019 => "0.019",
      0020 => "0.02",
      0021 => "0.021",
      0022 => "0.022",
      0023 => "0.023",
      0024 => "0.024",
      0025 => "0.025",
      0026 => "0.026",
      0027 => "0.027",
      0028 => "0.028",
      0029 => "0.029",
      0030 => "0.03",
      0031 => "0.031",
      0032 => "0.032",
      0033 => "0.033",
      0034 => "0.034",
      0035 => "0.035",
      0036 => "0.036",
      0037 => "0.037",
      0038 => "0.038",
      0039 => "0.039",
      0040 => "0.04",
      0041 => "0.041",
      0042 => "0.042",
      0043 => "0.043",
      0044 => "0.044",
      0045 => "0.045",
      0046 => "0.046",
      0047 => "0.047",
      0048 => "0.048",
      0049 => "0.049",
      0050 => "0.05",
      0051 => "0.051",
      0052 => "0.052",
      0053 => "0.053",
      0054 => "0.054",
      0055 => "0.055",
      0056 => "0.056",
      0057 => "0.057",
      0058 => "0.058",
      0059 => "0.059",
      0060 => "0.06",
      0061 => "0.061",
      0062 => "0.062",
      0063 => "0.063",
      0064 => "0.064",
      0065 => "0.065",
      0066 => "0.066",
      0067 => "0.067",
      0068 => "0.068",
      0069 => "0.069",
      0070 => "0.07",
      0071 => "0.071",
      0072 => "0.072",
      0073 => "0.073",
      0074 => "0.074",
      0075 => "0.075",
      0076 => "0.076",
      0077 => "0.077",
      0078 => "0.078",
      0079 => "0.079",
      0080 => "0.08",
      0081 => "0.081",
      0082 => "0.082",
      0083 => "0.083",
      0084 => "0.084",
      0085 => "0.085",
      0086 => "0.086",
      0087 => "0.087",
      0088 => "0.088",
      0089 => "0.089",
      0090 => "0.09",
      0091 => "0.091",
      0092 => "0.092",
      0093 => "0.093",
      0094 => "0.094",
      0095 => "0.095",
      0096 => "0.096",
      0097 => "0.097",
      0098 => "0.098",
      0099 => "0.099",
      0100 => "0.1",
      0101 => "0.101",
      0102 => "0.102",
      0103 => "0.103",
      0104 => "0.104",
      0105 => "0.105",
      0106 => "0.106",
      0107 => "0.107",
      0108 => "0.108",
      0109 => "0.109",
      0110 => "0.11",
      0111 => "0.111",
      0112 => "0.112",
      0113 => "0.113",
      0114 => "0.114",
      0115 => "0.115",
      0116 => "0.116",
      0117 => "0.117",
      0118 => "0.118",
      0119 => "0.119",
      0120 => "0.12",
      0121 => "0.121",
      0122 => "0.122",
      0123 => "0.123",
      0124 => "0.124",
      0125 => "0.125",
      0126 => "0.126",
      0127 => "0.127",
      0128 => "0.128",
      0129 => "0.129",
      0130 => "0.13",
      0131 => "0.131",
      0132 => "0.132",
      0133 => "0.133",
      0134 => "0.134",
      0135 => "0.135",
      0136 => "0.136",
      0137 => "0.137",
      0138 => "0.138",
      0139 => "0.139",
      0140 => "0.14",
      0141 => "0.141",
      0142 => "0.142",
      0143 => "0.143",
      0144 => "0.144",
      0145 => "0.145",
      0146 => "0.146",
      0147 => "0.147",
      0148 => "0.148",
      0149 => "0.149",
      0150 => "0.15",
      0151 => "0.151",
      0152 => "0.152",
      0153 => "0.153",
      0154 => "0.154",
      0155 => "0.155",
      0156 => "0.156",
      0157 => "0.157",
      0158 => "0.158",
      0159 => "0.159",
      0160 => "0.16",
      0161 => "0.161",
      0162 => "0.162",
      0163 => "0.163",
      0164 => "0.164",
      0165 => "0.165",
      0166 => "0.166",
      0167 => "0.167",
      0168 => "0.168",
      0169 => "0.169",
      0170 => "0.17",
      0171 => "0.171",
      0172 => "0.172",
      0173 => "0.173",
      0174 => "0.174",
      0175 => "0.175",
      0176 => "0.176",
      0177 => "0.177",
      0178 => "0.178",
      0179 => "0.179",
      0180 => "0.18",
      0181 => "0.181",
      0182 => "0.182",
      0183 => "0.183",
      0184 => "0.184",
      0185 => "0.185",
      0186 => "0.186",
      0187 => "0.187",
      0188 => "0.188",
      0189 => "0.189",
      0190 => "0.19",
      0191 => "0.191",
      0192 => "0.192",
      0193 => "0.193",
      0194 => "0.194",
      0195 => "0.195",
      0196 => "0.196",
      0197 => "0.197",
      0198 => "0.198",
      0199 => "0.199",
      0200 => "0.2",
      0201 => "0.201",
      0202 => "0.202",
      0203 => "0.203",
      0204 => "0.204",
      0205 => "0.205",
      0206 => "0.206",
      0207 => "0.207",
      0208 => "0.208",
      0209 => "0.209",
      0210 => "0.21",
      0211 => "0.211",
      0212 => "0.212",
      0213 => "0.213",
      0214 => "0.214",
      0215 => "0.215",
      0216 => "0.216",
      0217 => "0.217",
      0218 => "0.218",
      0219 => "0.219",
      0220 => "0.22",
      0221 => "0.221",
      0222 => "0.222",
      0223 => "0.223",
      0224 => "0.224",
      0225 => "0.225",
      0226 => "0.226",
      0227 => "0.227",
      0228 => "0.228",
      0229 => "0.229",
      0230 => "0.23",
      0231 => "0.231",
      0232 => "0.232",
      0233 => "0.233",
      0234 => "0.234",
      0235 => "0.235",
      0236 => "0.236",
      0237 => "0.237",
      0238 => "0.238",
      0239 => "0.239",
      0240 => "0.24",
      0241 => "0.241",
      0242 => "0.242",
      0243 => "0.243",
      0244 => "0.244",
      0245 => "0.245",
      0246 => "0.246",
      0247 => "0.247",
      0248 => "0.248",
      0249 => "0.249",
      0250 => "0.25",
      0251 => "0.251",
      0252 => "0.252",
      0253 => "0.253",
      0254 => "0.254",
      0255 => "0.255",
      0256 => "0.256",
      0257 => "0.257",
      0258 => "0.258",
      0259 => "0.259",
      0260 => "0.26",
      0261 => "0.261",
      0262 => "0.262",
      0263 => "0.263",
      0264 => "0.264",
      0265 => "0.265",
      0266 => "0.266",
      0267 => "0.267",
      0268 => "0.268",
      0269 => "0.269",
      0270 => "0.27",
      0271 => "0.271",
      0272 => "0.272",
      0273 => "0.273",
      0274 => "0.274",
      0275 => "0.275",
      0276 => "0.276",
      0277 => "0.277",
      0278 => "0.278",
      0279 => "0.279",
      0280 => "0.28",
      0281 => "0.281",
      0282 => "0.282",
      0283 => "0.283",
      0284 => "0.284",
      0285 => "0.285",
      0286 => "0.286",
      0287 => "0.287",
      0288 => "0.288",
      0289 => "0.289",
      0290 => "0.29",
      0291 => "0.291",
      0292 => "0.292",
      0293 => "0.293",
      0294 => "0.294",
      0295 => "0.295",
      0296 => "0.296",
      0297 => "0.297",
      0298 => "0.298",
      0299 => "0.299",
      0300 => "0.3",
      0301 => "0.301",
      0302 => "0.302",
      0303 => "0.303",
      0304 => "0.304",
      0305 => "0.305",
      0306 => "0.306",
      0307 => "0.307",
      0308 => "0.308",
      0309 => "0.309",
      0310 => "0.31",
      0311 => "0.311",
      0312 => "0.312",
      0313 => "0.313",
      0314 => "0.314",
      0315 => "0.315",
      0316 => "0.316",
      0317 => "0.317",
      0318 => "0.318",
      0319 => "0.319",
      0320 => "0.32",
      0321 => "0.321",
      0322 => "0.322",
      0323 => "0.323",
      0324 => "0.324",
      0325 => "0.325",
      0326 => "0.326",
      0327 => "0.327",
      0328 => "0.328",
      0329 => "0.329",
      0330 => "0.33",
      0331 => "0.331",
      0332 => "0.332",
      0333 => "0.333",
      0334 => "0.334",
      0335 => "0.335",
      0336 => "0.336",
      0337 => "0.337",
      0338 => "0.338",
      0339 => "0.339",
      0340 => "0.34",
      0341 => "0.341",
      0342 => "0.342",
      0343 => "0.343",
      0344 => "0.344",
      0345 => "0.345",
      0346 => "0.346",
      0347 => "0.347",
      0348 => "0.348",
      0349 => "0.349",
      0350 => "0.35",
      0351 => "0.351",
      0352 => "0.352",
      0353 => "0.353",
      0354 => "0.354",
      0355 => "0.355",
      0356 => "0.356",
      0357 => "0.357",
      0358 => "0.358",
      0359 => "0.359",
      0360 => "0.36",
      0361 => "0.361",
      0362 => "0.362",
      0363 => "0.363",
      0364 => "0.364",
      0365 => "0.365",
      0366 => "0.366",
      0367 => "0.367",
      0368 => "0.368",
      0369 => "0.369",
      0370 => "0.37",
      0371 => "0.371",
      0372 => "0.372",
      0373 => "0.373",
      0374 => "0.374",
      0375 => "0.375",
      0376 => "0.376",
      0377 => "0.377",
      0378 => "0.378",
      0379 => "0.379",
      0380 => "0.38",
      0381 => "0.381",
      0382 => "0.382",
      0383 => "0.383",
      0384 => "0.384",
      0385 => "0.385",
      0386 => "0.386",
      0387 => "0.387",
      0388 => "0.388",
      0389 => "0.389",
      0390 => "0.39",
      0391 => "0.391",
      0392 => "0.392",
      0393 => "0.393",
      0394 => "0.394",
      0395 => "0.395",
      0396 => "0.396",
      0397 => "0.397",
      0398 => "0.398",
      0399 => "0.399",
      0400 => "0.4",
      0401 => "0.401",
      0402 => "0.402",
      0403 => "0.403",
      0404 => "0.404",
      0405 => "0.405",
      0406 => "0.406",
      0407 => "0.407",
      0408 => "0.408",
      0409 => "0.409",
      0410 => "0.41",
      0411 => "0.411",
      0412 => "0.412",
      0413 => "0.413",
      0414 => "0.414",
      0415 => "0.415",
      0416 => "0.416",
      0417 => "0.417",
      0418 => "0.418",
      0419 => "0.419",
      0420 => "0.42",
      0421 => "0.421",
      0422 => "0.422",
      0423 => "0.423",
      0424 => "0.424",
      0425 => "0.425",
      0426 => "0.426",
      0427 => "0.427",
      0428 => "0.428",
      0429 => "0.429",
      0430 => "0.43",
      0431 => "0.431",
      0432 => "0.432",
      0433 => "0.433",
      0434 => "0.434",
      0435 => "0.435",
      0436 => "0.436",
      0437 => "0.437",
      0438 => "0.438",
      0439 => "0.439",
      0440 => "0.44",
      0441 => "0.441",
      0442 => "0.442",
      0443 => "0.443",
      0444 => "0.444",
      0445 => "0.445",
      0446 => "0.446",
      0447 => "0.447",
      0448 => "0.448",
      0449 => "0.449",
      0450 => "0.45",
      0451 => "0.451",
      0452 => "0.452",
      0453 => "0.453",
      0454 => "0.454",
      0455 => "0.455",
      0456 => "0.456",
      0457 => "0.457",
      0458 => "0.458",
      0459 => "0.459",
      0460 => "0.46",
      0461 => "0.461",
      0462 => "0.462",
      0463 => "0.463",
      0464 => "0.464",
      0465 => "0.465",
      0466 => "0.466",
      0467 => "0.467",
      0468 => "0.468",
      0469 => "0.469",
      0470 => "0.47",
      0471 => "0.471",
      0472 => "0.472",
      0473 => "0.473",
      0474 => "0.474",
      0475 => "0.475",
      0476 => "0.476",
      0477 => "0.477",
      0478 => "0.478",
      0479 => "0.479",
      0480 => "0.48",
      0481 => "0.481",
      0482 => "0.482",
      0483 => "0.483",
      0484 => "0.484",
      0485 => "0.485",
      0486 => "0.486",
      0487 => "0.487",
      0488 => "0.488",
      0489 => "0.489",
      0490 => "0.49",
      0491 => "0.491",
      0492 => "0.492",
      0493 => "0.493",
      0494 => "0.494",
      0495 => "0.495",
      0496 => "0.496",
      0497 => "0.497",
      0498 => "0.498",
      0499 => "0.499",
      0500 => "0.5",
      0501 => "0.501",
      0502 => "0.502",
      0503 => "0.503",
      0504 => "0.504",
      0505 => "0.505",
      0506 => "0.506",
      0507 => "0.507",
      0508 => "0.508",
      0509 => "0.509",
      0510 => "0.51",
      0511 => "0.511",
      0512 => "0.512",
      0513 => "0.513",
      0514 => "0.514",
      0515 => "0.515",
      0516 => "0.516",
      0517 => "0.517",
      0518 => "0.518",
      0519 => "0.519",
      0520 => "0.52",
      0521 => "0.521",
      0522 => "0.522",
      0523 => "0.523",
      0524 => "0.524",
      0525 => "0.525",
      0526 => "0.526",
      0527 => "0.527",
      0528 => "0.528",
      0529 => "0.529",
      0530 => "0.53",
      0531 => "0.531",
      0532 => "0.532",
      0533 => "0.533",
      0534 => "0.534",
      0535 => "0.535",
      0536 => "0.536",
      0537 => "0.537",
      0538 => "0.538",
      0539 => "0.539",
      0540 => "0.54",
      0541 => "0.541",
      0542 => "0.542",
      0543 => "0.543",
      0544 => "0.544",
      0545 => "0.545",
      0546 => "0.546",
      0547 => "0.547",
      0548 => "0.548",
      0549 => "0.549",
      0550 => "0.55",
      0551 => "0.551",
      0552 => "0.552",
      0553 => "0.553",
      0554 => "0.554",
      0555 => "0.555",
      0556 => "0.556",
      0557 => "0.557",
      0558 => "0.558",
      0559 => "0.559",
      0560 => "0.56",
      0561 => "0.561",
      0562 => "0.562",
      0563 => "0.563",
      0564 => "0.564",
      0565 => "0.565",
      0566 => "0.566",
      0567 => "0.567",
      0568 => "0.568",
      0569 => "0.569",
      0570 => "0.57",
      0571 => "0.571",
      0572 => "0.572",
      0573 => "0.573",
      0574 => "0.574",
      0575 => "0.575",
      0576 => "0.576",
      0577 => "0.577",
      0578 => "0.578",
      0579 => "0.579",
      0580 => "0.58",
      0581 => "0.581",
      0582 => "0.582",
      0583 => "0.583",
      0584 => "0.584",
      0585 => "0.585",
      0586 => "0.586",
      0587 => "0.587",
      0588 => "0.588",
      0589 => "0.589",
      0590 => "0.59",
      0591 => "0.591",
      0592 => "0.592",
      0593 => "0.593",
      0594 => "0.594",
      0595 => "0.595",
      0596 => "0.596",
      0597 => "0.597",
      0598 => "0.598",
      0599 => "0.599",
      0600 => "0.6",
      0601 => "0.601",
      0602 => "0.602",
      0603 => "0.603",
      0604 => "0.604",
      0605 => "0.605",
      0606 => "0.606",
      0607 => "0.607",
      0608 => "0.608",
      0609 => "0.609",
      0610 => "0.61",
      0611 => "0.611",
      0612 => "0.612",
      0613 => "0.613",
      0614 => "0.614",
      0615 => "0.615",
      0616 => "0.616",
      0617 => "0.617",
      0618 => "0.618",
      0619 => "0.619",
      0620 => "0.62",
      0621 => "0.621",
      0622 => "0.622",
      0623 => "0.623",
      0624 => "0.624",
      0625 => "0.625",
      0626 => "0.626",
      0627 => "0.627",
      0628 => "0.628",
      0629 => "0.629",
      0630 => "0.63",
      0631 => "0.631",
      0632 => "0.632",
      0633 => "0.633",
      0634 => "0.634",
      0635 => "0.635",
      0636 => "0.636",
      0637 => "0.637",
      0638 => "0.638",
      0639 => "0.639",
      0640 => "0.64",
      0641 => "0.641",
      0642 => "0.642",
      0643 => "0.643",
      0644 => "0.644",
      0645 => "0.645",
      0646 => "0.646",
      0647 => "0.647",
      0648 => "0.648",
      0649 => "0.649",
      0650 => "0.65",
      0651 => "0.651",
      0652 => "0.652",
      0653 => "0.653",
      0654 => "0.654",
      0655 => "0.655",
      0656 => "0.656",
      0657 => "0.657",
      0658 => "0.658",
      0659 => "0.659",
      0660 => "0.66",
      0661 => "0.661",
      0662 => "0.662",
      0663 => "0.663",
      0664 => "0.664",
      0665 => "0.665",
      0666 => "0.666",
      0667 => "0.667",
      0668 => "0.668",
      0669 => "0.669",
      0670 => "0.67",
      0671 => "0.671",
      0672 => "0.672",
      0673 => "0.673",
      0674 => "0.674",
      0675 => "0.675",
      0676 => "0.676",
      0677 => "0.677",
      0678 => "0.678",
      0679 => "0.679",
      0680 => "0.68",
      0681 => "0.681",
      0682 => "0.682",
      0683 => "0.683",
      0684 => "0.684",
      0685 => "0.685",
      0686 => "0.686",
      0687 => "0.687",
      0688 => "0.688",
      0689 => "0.689",
      0690 => "0.69",
      0691 => "0.691",
      0692 => "0.692",
      0693 => "0.693",
      0694 => "0.694",
      0695 => "0.695",
      0696 => "0.696",
      0697 => "0.697",
      0698 => "0.698",
      0699 => "0.699",
      0700 => "0.7",
      0701 => "0.701",
      0702 => "0.702",
      0703 => "0.703",
      0704 => "0.704",
      0705 => "0.705",
      0706 => "0.706",
      0707 => "0.707",
      0708 => "0.708",
      0709 => "0.709",
      0710 => "0.71",
      0711 => "0.711",
      0712 => "0.712",
      0713 => "0.713",
      0714 => "0.714",
      0715 => "0.715",
      0716 => "0.716",
      0717 => "0.717",
      0718 => "0.718",
      0719 => "0.719",
      0720 => "0.72",
      0721 => "0.721",
      0722 => "0.722",
      0723 => "0.723",
      0724 => "0.724",
      0725 => "0.725",
      0726 => "0.726",
      0727 => "0.727",
      0728 => "0.728",
      0729 => "0.729",
      0730 => "0.73",
      0731 => "0.731",
      0732 => "0.732",
      0733 => "0.733",
      0734 => "0.734",
      0735 => "0.735",
      0736 => "0.736",
      0737 => "0.737",
      0738 => "0.738",
      0739 => "0.739",
      0740 => "0.74",
      0741 => "0.741",
      0742 => "0.742",
      0743 => "0.743",
      0744 => "0.744",
      0745 => "0.745",
      0746 => "0.746",
      0747 => "0.747",
      0748 => "0.748",
      0749 => "0.749",
      0750 => "0.75",
      0751 => "0.751",
      0752 => "0.752",
      0753 => "0.753",
      0754 => "0.754",
      0755 => "0.755",
      0756 => "0.756",
      0757 => "0.757",
      0758 => "0.758",
      0759 => "0.759",
      0760 => "0.76",
      0761 => "0.761",
      0762 => "0.762",
      0763 => "0.763",
      0764 => "0.764",
      0765 => "0.765",
      0766 => "0.766",
      0767 => "0.767",
      0768 => "0.768",
      0769 => "0.769",
      0770 => "0.77",
      0771 => "0.771",
      0772 => "0.772",
      0773 => "0.773",
      0774 => "0.774",
      0775 => "0.775",
      0776 => "0.776",
      0777 => "0.777",
      0778 => "0.778",
      0779 => "0.779",
      0780 => "0.78",
      0781 => "0.781",
      0782 => "0.782",
      0783 => "0.783",
      0784 => "0.784",
      0785 => "0.785",
      0786 => "0.786",
      0787 => "0.787",
      0788 => "0.788",
      0789 => "0.789",
      0790 => "0.79",
      0791 => "0.791",
      0792 => "0.792",
      0793 => "0.793",
      0794 => "0.794",
      0795 => "0.795",
      0796 => "0.796",
      0797 => "0.797",
      0798 => "0.798",
      0799 => "0.799",
      0800 => "0.8",
      0801 => "0.801",
      0802 => "0.802",
      0803 => "0.803",
      0804 => "0.804",
      0805 => "0.805",
      0806 => "0.806",
      0807 => "0.807",
      0808 => "0.808",
      0809 => "0.809",
      0810 => "0.81",
      0811 => "0.811",
      0812 => "0.812",
      0813 => "0.813",
      0814 => "0.814",
      0815 => "0.815",
      0816 => "0.816",
      0817 => "0.817",
      0818 => "0.818",
      0819 => "0.819",
      0820 => "0.82",
      0821 => "0.821",
      0822 => "0.822",
      0823 => "0.823",
      0824 => "0.824",
      0825 => "0.825",
      0826 => "0.826",
      0827 => "0.827",
      0828 => "0.828",
      0829 => "0.829",
      0830 => "0.83",
      0831 => "0.831",
      0832 => "0.832",
      0833 => "0.833",
      0834 => "0.834",
      0835 => "0.835",
      0836 => "0.836",
      0837 => "0.837",
      0838 => "0.838",
      0839 => "0.839",
      0840 => "0.84",
      0841 => "0.841",
      0842 => "0.842",
      0843 => "0.843",
      0844 => "0.844",
      0845 => "0.845",
      0846 => "0.846",
      0847 => "0.847",
      0848 => "0.848",
      0849 => "0.849",
      0850 => "0.85",
      0851 => "0.851",
      0852 => "0.852",
      0853 => "0.853",
      0854 => "0.854",
      0855 => "0.855",
      0856 => "0.856",
      0857 => "0.857",
      0858 => "0.858",
      0859 => "0.859",
      0860 => "0.86",
      0861 => "0.861",
      0862 => "0.862",
      0863 => "0.863",
      0864 => "0.864",
      0865 => "0.865",
      0866 => "0.866",
      0867 => "0.867",
      0868 => "0.868",
      0869 => "0.869",
      0870 => "0.87",
      0871 => "0.871",
      0872 => "0.872",
      0873 => "0.873",
      0874 => "0.874",
      0875 => "0.875",
      0876 => "0.876",
      0877 => "0.877",
      0878 => "0.878",
      0879 => "0.879",
      0880 => "0.88",
      0881 => "0.881",
      0882 => "0.882",
      0883 => "0.883",
      0884 => "0.884",
      0885 => "0.885",
      0886 => "0.886",
      0887 => "0.887",
      0888 => "0.888",
      0889 => "0.889",
      0890 => "0.89",
      0891 => "0.891",
      0892 => "0.892",
      0893 => "0.893",
      0894 => "0.894",
      0895 => "0.895",
      0896 => "0.896",
      0897 => "0.897",
      0898 => "0.898",
      0899 => "0.899",
      0900 => "0.9",
      0901 => "0.901",
      0902 => "0.902",
      0903 => "0.903",
      0904 => "0.904",
      0905 => "0.905",
      0906 => "0.906",
      0907 => "0.907",
      0908 => "0.908",
      0909 => "0.909",
      0910 => "0.91",
      0911 => "0.911",
      0912 => "0.912",
      0913 => "0.913",
      0914 => "0.914",
      0915 => "0.915",
      0916 => "0.916",
      0917 => "0.917",
      0918 => "0.918",
      0919 => "0.919",
      0920 => "0.92",
      0921 => "0.921",
      0922 => "0.922",
      0923 => "0.923",
      0924 => "0.924",
      0925 => "0.925",
      0926 => "0.926",
      0927 => "0.927",
      0928 => "0.928",
      0929 => "0.929",
      0930 => "0.93",
      0931 => "0.931",
      0932 => "0.932",
      0933 => "0.933",
      0934 => "0.934",
      0935 => "0.935",
      0936 => "0.936",
      0937 => "0.937",
      0938 => "0.938",
      0939 => "0.939",
      0940 => "0.94",
      0941 => "0.941",
      0942 => "0.942",
      0943 => "0.943",
      0944 => "0.944",
      0945 => "0.945",
      0946 => "0.946",
      0947 => "0.947",
      0948 => "0.948",
      0949 => "0.949",
      0950 => "0.95",
      0951 => "0.951",
      0952 => "0.952",
      0953 => "0.953",
      0954 => "0.954",
      0955 => "0.955",
      0956 => "0.956",
      0957 => "0.957",
      0958 => "0.958",
      0959 => "0.959",
      0960 => "0.96",
      0961 => "0.961",
      0962 => "0.962",
      0963 => "0.963",
      0964 => "0.964",
      0965 => "0.965",
      0966 => "0.966",
      0967 => "0.967",
      0968 => "0.968",
      0969 => "0.969",
      0970 => "0.97",
      0971 => "0.971",
      0972 => "0.972",
      0973 => "0.973",
      0974 => "0.974",
      0975 => "0.975",
      0976 => "0.976",
      0977 => "0.977",
      0978 => "0.978",
      0979 => "0.979",
      0980 => "0.98",
      0981 => "0.981",
      0982 => "0.982",
      0983 => "0.983",
      0984 => "0.984",
      0985 => "0.985",
      0986 => "0.986",
      0987 => "0.987",
      0988 => "0.988",
      0989 => "0.989",
      0990 => "0.99",
      0991 => "0.991",
      0992 => "0.992",
      0993 => "0.993",
      0994 => "0.994",
      0995 => "0.995",
      0996 => "0.996",
      0997 => "0.997",
      0998 => "0.998",
      0999 => "0.999",
      1000 => "1.0",
      _ => unreachable!(),
    }
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

///
/// Represents one part of an accept mime
/// # See
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept
#[derive(Clone, PartialEq, Debug, Eq)]
pub struct AcceptMime {
  value: Option<Either<MimeGroup, MimeType>>,
  q: QValue,
}

impl PartialOrd<Self> for AcceptMime {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for AcceptMime {
  fn cmp(&self, other: &Self) -> Ordering {
    other.q.cmp(&self.q)
  }
}

impl AcceptMime {
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

        let qvalue = match QValue::parse(&rawq[2..]) {
          Some(qvalue) => qvalue,
          None => return None,
        };

        if mime == "*/*" {
          data.push(AcceptMime { value: None, q: qvalue });
          continue;
        }

        match MimeType::parse(mime) {
          None => match MimeGroup::parse(mime) {
            Some(group) => {
              if &mime[group.as_str().len()..] != "/*" {
                return None;
              }
              data.push(AcceptMime { value: Some(Either::Left(group)), q: qvalue })
            }
            None => return None,
          },
          Some(mime) => data.push(AcceptMime { value: Some(Either::Right(mime)), q: qvalue }),
        };

        continue;
      }

      if mime == "*/*" {
        data.push(AcceptMime { value: None, q: QValue::default() });
        continue;
      }

      match MimeType::parse(mime) {
        None => match MimeGroup::parse(mime) {
          Some(group) => {
            if &mime[group.as_str().len()..] != "/*" {
              return None;
            }
            data.push(AcceptMime { value: Some(Either::Left(group)), q: QValue::default() })
          }
          None => return None,
        },
        Some(mime) => {
          data.push(AcceptMime { value: Some(Either::Right(mime)), q: QValue::default() })
        }
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

  /// Is this a */* accept?
  pub const fn is_wildcard(&self) -> bool {
    self.value.is_none()
  }

  /// Get the QValue of this accept mime.
  pub const fn qvalue(&self) -> QValue {
    self.q
  }

  /// Is this a group wildcard? i.e: `video/*` or `text/*`
  pub const fn is_group_wildcard(&self) -> bool {
    matches!(self.value, Some(Left(_)))
  }

  /// Is this a non wildcard mime? i.e: `video/mp4`
  pub const fn is_mime(&self) -> bool {
    matches!(self.value, Some(Right(_)))
  }

  /// Get the mime type. returns none if this is any type of wildcard mime
  pub const fn mime(&self) -> Option<&MimeType> {
    match &self.value {
      Some(Right(mime)) => Some(mime),
      _ => None,
    }
  }

  /// Get the mime type. returns none if this is the `*/*` mime.
  pub const fn group(&self) -> Option<&MimeGroup> {
    match &self.value {
      Some(Right(mime)) => Some(mime.mime_group()),
      Some(Left(group)) => Some(group),
      _ => None,
    }
  }

  /// Returns a AcceptMime equivalent to calling parse with `*/*`
  pub const fn wildcard(q: QValue) -> AcceptMime {
    AcceptMime { value: None, q }
  }

  /// Returns a AcceptMime equivalent to calling parse with `group/*` depending on MimeGroup.
  pub const fn from_group(group: MimeGroup, q: QValue) -> AcceptMime {
    AcceptMime { value: Some(Left(group)), q }
  }

  /// Returns a AcceptMime equivalent to calling parse with `group/type` depending on MimeType.
  pub const fn from_mime(mime: MimeType, q: QValue) -> AcceptMime {
    AcceptMime { value: Some(Right(mime)), q }
  }
}

impl Default for AcceptMime {
  fn default() -> Self {
    AcceptMime::wildcard(QValue::default())
  }
}

impl Display for AcceptMime {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match &self.value {
      Some(Left(group)) => {
        f.write_str(group.as_str())?;
        f.write_str("/*")?;
      }
      Some(Right(mime)) => {
        f.write_str(mime.as_str())?;
      }
      None => f.write_str("*/*")?,
    }

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
  /// Any human or pseudo human readable text.
  Text,
  /// Anything else.
  Other(String),
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
      MimeType::TextLua => &MimeGroup::Application,
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
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
