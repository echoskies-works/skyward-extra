//! The icons the pages draw, as the stock Settings draws its own: a tile in
//! one colour whose corners ease into its sides, and a white glyph over it
//! whose details are cut out of filled shapes. Each is an outline in the
//! stock icons' 28-unit box, written as SVG path data. Where the stock
//! Settings has an icon for the same thing, its glyph is the stock one;
//! the rest are drawn here in their manner, their lines as heavy as the
//! stock WLAN arcs and round at their ends.

use crate::canvas::Canvas;
use crate::color::{mix, Rgba};
use crate::theme::Theme;
use std::sync::{Arc, Mutex};
use svgtypes::{SimplePathSegment, SimplifyingPathParser};
use tiny_skia::{BlendMode, Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// An arrow pointing up, as the stock Settings' updater shows it.
    Install,
    /// Two sheets, the one behind with its corner turned, as the stock
    /// Settings' accounts and backup show them.
    Backup,
    /// A ring running back round to where it started, with a clock's
    /// hands inside.
    Restore,
    /// A memory card, its corner cut and its contacts along its top.
    Mount,
    /// A bin with two slots cut in it, its lid and the lid's handle.
    Wipe,
    /// A nut, as the stock Settings' app settings show it.
    Gear,
    /// The power sign: a ring open at the top, a bar through the opening.
    Power,
    /// A globe, as the stock Settings' VPN shows it.
    Globe,
    /// A clock face with its hands cut out of it.
    Clock,
    /// A calculator: its body, the screen across the top and the keys under
    /// it. Project Aether's own.
    Calculator,
    /// A window with a prompt cut out of it.
    Terminal,
    /// A plug: its metal end with two holes, its body and its cable.
    Usb,
    /// A flash drive: its metal end with two holes, and its body with a
    /// light.
    UsbDrive,
    /// Two layers, one over the other, as the stock Settings' service
    /// assistant shows them.
    Layers,
    /// A phone, as the stock Settings' own device shows it.
    Phone,
    /// A padlock, as the stock Settings' lock screen shows it.
    Lock,
    /// Three dots in a circle, as the stock Settings' more settings show
    /// them.
    More,
    /// A folder, as the stock Settings shows file transfer.
    Folder,
    /// A page with three lines of text cut out of it.
    Log,
    /// A shield with a tick, as the stock Settings' security status shows
    /// it.
    Shield,
    /// The WLAN's fan, as the stock Settings' WLAN shows it.
    Wlan,
    /// A battery, as the stock Settings' battery shows it.
    Battery,
    /// Two phones, the one behind showing round the one in front.
    Systems,
    /// A disc, as a player shows one: its face, the ring at its centre and
    /// the two arcs across it.
    Music,
    /// A globe stood on its rings, as a browser shows one.
    Browser,
    /// A monitor on its foot, as a tasks page shows one.
    Tasks,
    /// A sun behind a cloud: what a weather page shows.
    Weather,
    /// A triangle: the stopwatch's key that sets the count going.
    Play,
    /// Two bars: the stopwatch's key that stops the count where it is.
    Pause,
    /// A flag: the stopwatch's key that takes an entry on the paper.
    Flag,
    /// A ring with its head turning back on itself: the stopwatch's key that
    /// puts the watch back to nothing.
    Replay,
    /// A bar: the timer's key that takes a minute off the length.
    Minus,
    /// Two bars crossed: the timer's key that puts a minute on it.
    Plus,
    /// A toothed wheel with a round hole: the gear the clock's settings
    /// button carries, where the nut above would read as a nut.
    Settings,
}

/// The stock icons' box: every outline here is laid out in it.
const BOX: f32 = 28.0;

/// How a part of a glyph is drawn.
#[derive(Clone, Copy)]
enum Shape {
    /// Filled wherever the outline goes round.
    Fill(&'static str),
    /// Filled where the outline goes round an odd number of times, which
    /// leaves out what lies inside it twice: how the stock glyphs cut their
    /// details out of their shapes.
    EvenOdd(&'static str),
    /// A line this wide, round at its ends and where it turns.
    Line(&'static str, f32),
}

/// A part of a glyph, drawn over the parts before it or cut out of them.
#[derive(Clone, Copy)]
struct Part {
    shape: Shape,
    cut: bool,
}

const fn add(shape: Shape) -> Part {
    Part { shape, cut: false }
}

const fn cut(shape: Shape) -> Part {
    Part { shape, cut: true }
}

/// The stock tile, which every one of the stock Settings' icons is drawn
/// on.
const TILE: &[Part] = &[add(Shape::Fill(
    "M0,11.952C0,7.768 0,5.677 0.814,4.079C1.53,2.673 2.673,1.53 4.079,0.814C5.677,0 7.768,0 11.952,0H16.048C20.232,0 22.323,0 23.921,0.814\
     C25.327,1.53 26.47,2.673 27.186,4.079C28,5.677 28,7.768 28,11.952V16.048C28,20.232 28,22.323 27.186,23.921C26.47,25.327 25.327,26.47 23.921,27.186\
     C22.323,28 20.232,28 16.048,28H11.952C7.768,28 5.677,28 4.079,27.186C2.673,26.47 1.53,25.327 0.814,23.921C0,22.323 0,20.232 0,16.048V11.952Z",
))];

const WLAN: &[Part] = &[add(Shape::Fill(
    "M11.305,19.161C11.117,18.97 11.023,18.874 10.981,18.739C10.948,18.635 10.949,18.477 10.983,18.374C11.028,18.239 11.104,18.164 11.256,18.014\
     C11.961,17.318 12.93,16.888 13.999,16.888C15.072,16.888 16.043,17.321 16.749,18.021C16.9,18.172 16.976,18.247 17.02,18.382\
     C17.054,18.485 17.055,18.643 17.022,18.746C16.979,18.882 16.885,18.977 16.697,19.167L14.947,20.943C14.616,21.279 14.45,21.447 14.259,21.51\
     C14.091,21.565 13.909,21.565 13.741,21.51C13.549,21.446 13.384,21.278 13.053,20.941L11.305,19.161Z\
     M8.031,15.827C7.819,15.611 7.713,15.503 7.67,15.366C7.635,15.254 7.637,15.11 7.675,14.999C7.722,14.863 7.82,14.769 8.017,14.579\
     C9.567,13.085 11.675,12.165 13.999,12.165C16.33,12.165 18.443,13.09 19.995,14.592C20.191,14.783 20.289,14.877 20.335,15.013\
     C20.373,15.124 20.375,15.268 20.339,15.381C20.296,15.517 20.19,15.625 19.978,15.84L19.743,16.079C19.484,16.341 19.354,16.473 19.224,16.518\
     C19.096,16.562 18.998,16.565 18.868,16.528C18.736,16.491 18.577,16.348 18.259,16.061C17.131,15.044 15.638,14.424 13.999,14.424\
     C12.365,14.424 10.876,15.04 9.749,16.051C9.431,16.337 9.272,16.48 9.139,16.517C9.009,16.554 8.911,16.551 8.783,16.506\
     C8.653,16.461 8.524,16.329 8.265,16.066L8.031,15.827Z\
     M4.731,12.466C4.512,12.243 4.402,12.131 4.36,11.995C4.324,11.88 4.326,11.741 4.365,11.627C4.411,11.492 4.516,11.391 4.727,11.187\
     C7.128,8.868 10.397,7.441 13.999,7.441C17.612,7.441 20.888,8.876 23.291,11.208C23.501,11.411 23.607,11.513 23.652,11.648\
     C23.691,11.762 23.693,11.901 23.656,12.016C23.614,12.152 23.504,12.264 23.284,12.486L23.035,12.739C22.787,12.991 22.663,13.116 22.53,13.161\
     C22.407,13.203 22.295,13.205 22.169,13.168C22.035,13.127 21.896,12.995 21.617,12.732C19.63,10.852 16.949,9.7 13.999,9.7\
     C11.057,9.7 8.383,10.845 6.398,12.715C6.119,12.978 5.979,13.11 5.844,13.15C5.719,13.187 5.607,13.185 5.483,13.143\
     C5.351,13.097 5.227,12.971 4.979,12.719L4.731,12.466Z",
))];

const INSTALL: &[Part] = &[add(Shape::EvenOdd(
    "M12.84,5.931L7.803,11.9C7.131,12.696 6.796,13.093 6.789,13.43C6.784,13.722 6.913,14.001 7.14,14.185C7.401,14.397 7.922,14.397 8.963,14.397\
     H10.403C10.54,14.397 10.648,14.512 10.645,14.648C10.64,14.841 10.64,15.064 10.64,15.326V20.391C10.64,21.277 10.64,21.72 10.813,22.058\
     C10.964,22.355 11.206,22.597 11.504,22.749C11.842,22.921 12.285,22.921 13.17,22.921H14.831C15.717,22.921 16.159,22.921 16.497,22.749\
     C16.795,22.597 17.037,22.355 17.189,22.058C17.361,21.72 17.361,21.277 17.361,20.391V15.326C17.361,15.064 17.361,14.841 17.357,14.648\
     C17.353,14.512 17.462,14.397 17.598,14.397H19.037C20.078,14.397 20.599,14.397 20.86,14.185C21.087,14.001 21.216,13.722 21.211,13.43\
     C21.204,13.093 20.869,12.696 20.197,11.9L15.16,5.931C14.761,5.458 14.562,5.221 14.323,5.135C14.115,5.059 13.885,5.059 13.677,5.135\
     C13.438,5.221 13.239,5.458 12.84,5.931Z",
))];

const BACKUP: &[Part] = &[add(Shape::EvenOdd(
    "M15.328,6.556C15.817,6.556 15.958,6.56 16.085,6.589C16.213,6.618 16.336,6.665 16.45,6.73C16.563,6.795 16.67,6.887 17.032,7.217\
     L20.054,9.972C20.464,10.347 20.58,10.459 20.66,10.581C20.741,10.704 20.801,10.84 20.838,10.983C20.874,11.125 20.879,11.285 20.879,11.841\
     V17.317C20.879,17.571 21.085,17.777 21.339,17.777H21.774C22.028,17.777 22.234,17.571 22.234,17.317V11.841L22.234,11.768\
     C22.234,11.318 22.235,10.976 22.151,10.648C22.077,10.359 21.956,10.085 21.792,9.836C21.607,9.554 21.354,9.323 21.021,9.02L20.967,8.971\
     L17.945,6.215L17.897,6.172C17.604,5.904 17.381,5.701 17.12,5.552C16.89,5.421 16.642,5.325 16.383,5.267C16.091,5.201 15.789,5.201 15.392,5.201\
     L15.328,5.201H11.998C11.744,5.201 11.538,5.407 11.538,5.661V6.096C11.538,6.35 11.744,6.556 11.998,6.556H15.328Z\
     M7.485,9.053C7.216,9.58 7.216,10.27 7.216,11.651V18.857C7.216,20.237 7.216,20.927 7.485,21.454C7.721,21.918 8.098,22.295 8.562,22.531\
     C9.089,22.8 9.779,22.8 11.159,22.8H15.785C17.165,22.8 17.855,22.8 18.382,22.531C18.846,22.295 19.223,21.918 19.459,21.454\
     C19.728,20.927 19.728,20.237 19.728,18.857V12.763C19.728,12.33 19.728,12.113 19.675,11.911C19.629,11.732 19.553,11.562 19.45,11.408\
     C19.334,11.235 19.173,11.091 18.85,10.802L18.85,10.802L16.132,8.376L16.132,8.376C15.855,8.129 15.717,8.005 15.559,7.917\
     C15.42,7.839 15.27,7.782 15.113,7.747C14.938,7.708 14.752,7.708 14.381,7.708H11.159C9.779,7.708 9.089,7.708 8.562,7.976\
     C8.098,8.213 7.721,8.59 7.485,9.053Z",
))];

const RESTORE: &[Part] = &[
    add(Shape::Line("M8.25,11.1A7.3,7.3 0 1 1 8.3,17.6", 2.3)),
    // The arrowhead, its corners rounded by a thin line round it.
    add(Shape::Fill("M4.95,9.2L10.65,9.55L7.95,13.95Z")),
    add(Shape::Line("M4.95,9.2L10.65,9.55L7.95,13.95Z", 1.0)),
    add(Shape::Line("M14.9,10.2V14.3L17.6,15.9", 2.1)),
];

const MOUNT: &[Part] = &[
    // The card's cut corner is rounded as its other corners are, by a thin
    // line round its edge.
    add(Shape::Fill("M11.6,5.4H18.3C19.29,5.4 20.1,6.21 20.1,7.2V20.8C20.1,21.79 19.29,22.6 18.3,22.6H9.7C8.71,22.6 7.9,21.79 7.9,20.8V9.1Z")),
    add(Shape::Line("M11.6,5.4H18.3C19.29,5.4 20.1,6.21 20.1,7.2V20.8C20.1,21.79 19.29,22.6 18.3,22.6H9.7C8.71,22.6 7.9,21.79 7.9,20.8V9.1Z", 1.0)),
    cut(Shape::Line("M12,8.2V10.9", 1.5)),
    cut(Shape::Line("M14.65,8.2V10.9", 1.5)),
    cut(Shape::Line("M17.3,8.2V10.9", 1.5)),
];

const WIPE: &[Part] = &[
    add(Shape::Line("M7.2,8.6H20.8", 2.25)),
    add(Shape::Line("M11.4,8.2V6.6C11.4,5.83 11.83,5.4 12.6,5.4H15.4C16.17,5.4 16.6,5.83 16.6,6.6V8.2", 2.0)),
    // The bin, its top corners rounded by a thin line round it.
    add(Shape::Fill("M8.6,11.1H19.4L18.55,21.05C18.43,22.45 17.6,23.1 16.2,23.1H11.8C10.4,23.1 9.57,22.45 9.45,21.05Z")),
    add(Shape::Line("M8.6,11.1H19.4L18.55,21.05C18.43,22.45 17.6,23.1 16.2,23.1H11.8C10.4,23.1 9.57,22.45 9.45,21.05Z", 0.8)),
    cut(Shape::Line("M12.2,14.2V19.9", 1.6)),
    cut(Shape::Line("M15.8,14.2V19.9", 1.6)),
];

const GEAR: &[Part] = &[
    add(Shape::Fill(
        "M16.435,14C16.435,15.345 15.345,16.436 14,16.436C12.655,16.436 11.565,15.345 11.565,14C11.565,12.655 12.655,11.565 14,11.565\
         C15.345,11.565 16.435,12.655 16.435,14Z",
    )),
    add(Shape::EvenOdd(
        "M13.523,5.539C13.168,5.613 12.833,5.802 12.163,6.18L8.049,8.5C7.356,8.89 7.009,9.086 6.757,9.362C6.534,9.606 6.365,9.895 6.262,10.209\
         C6.146,10.564 6.146,10.962 6.146,11.757V16.244C6.146,17.04 6.146,17.438 6.262,17.793C6.365,18.107 6.534,18.396 6.757,18.64\
         C7.009,18.916 7.356,19.112 8.049,19.502L12.163,21.822C12.833,22.2 13.168,22.389 13.523,22.462C13.838,22.528 14.162,22.528 14.476,22.462\
         C14.832,22.389 15.167,22.2 15.837,21.822L19.951,19.502C20.644,19.112 20.99,18.916 21.243,18.64C21.466,18.396 21.635,18.107 21.738,17.793\
         C21.854,17.438 21.854,17.04 21.854,16.244V11.757C21.854,10.962 21.854,10.564 21.738,10.209C21.635,9.895 21.466,9.606 21.243,9.362\
         C20.99,9.086 20.644,8.89 19.951,8.5L15.837,6.18C15.167,5.802 14.832,5.613 14.476,5.539C14.162,5.474 13.838,5.474 13.523,5.539Z\
         M18.236,14C18.236,16.34 16.339,18.236 14,18.236C11.661,18.236 9.764,16.34 9.764,14C9.764,11.661 11.661,9.765 14,9.765\
         C16.339,9.765 18.236,11.661 18.236,14Z",
    )),
];

/// The gear the clock's settings button carries, cut from the user's own
/// SVG: a toothed wheel with a round hole. [`GEAR`] above is a nut, which
/// the eye takes for a gear on a tile sixteen dp across and more but not on
/// a bar button, where its teeth run together; this one keeps its teeth at
/// that size.
const SETTINGS: &[Part] = &[add(Shape::EvenOdd(
    "M14.001,18.049C15.116,18.049,16.07,17.654,16.861,16.862\
     C17.653,16.07,18.049,15.117,18.049,14.001C18.049,12.886,17.653,11.932,16.861,11.14\
     C16.07,10.349,15.116,9.953,14.001,9.953C12.885,9.953,11.931,10.349,11.14,11.14\
     C10.348,11.932,9.952,12.886,9.952,14.001C9.952,15.117,10.348,16.07,11.14,16.862\
     C11.931,17.654,12.885,18.049,14.001,18.049ZM22.584,15.133L25.013,17.023\
     C25.265,17.203,25.301,17.455,25.122,17.779L22.8,21.774\
     C22.655,22.026,22.422,22.098,22.098,21.991L19.237,20.857\
     C18.482,21.397,17.834,21.775,17.293,21.991L16.861,25.014\
     C16.789,25.338,16.609,25.5,16.321,25.5L11.679,25.5C11.391,25.5,11.211,25.338,11.139,25.014\
     L10.707,21.991C10.023,21.703,9.374,21.325,8.763,20.857L5.902,21.991\
     C5.578,22.099,5.344,22.027,5.2,21.774L2.878,17.779C2.699,17.455,2.735,17.203,2.987,17.023\
     L5.416,15.133C5.38,14.881,5.361,14.504,5.361,14C5.361,13.496,5.38,13.119,5.416,12.867\
     L2.987,10.977C2.735,10.797,2.699,10.545,2.878,10.221L5.2,6.226\
     C5.345,5.974,5.578,5.902,5.902,6.009L8.763,7.143C9.518,6.603,10.166,6.225,10.707,6.009\
     L11.139,2.986C11.211,2.662,11.391,2.5,11.679,2.5L16.321,2.5\
     C16.609,2.5,16.789,2.662,16.861,2.986L17.293,6.009C17.977,6.297,18.626,6.675,19.237,7.143\
     L22.098,6.009C22.422,5.901,22.656,5.973,22.8,6.226L25.122,10.221\
     C25.301,10.545,25.265,10.797,25.013,10.977L22.584,12.867\
     C22.62,13.119,22.639,13.496,22.639,14C22.639,14.504,22.62,14.881,22.584,15.133Z",
))];

const POWER: &[Part] = &[add(Shape::Line("M10.14,9.48A6.9,6.9 0 1 0 17.86,9.48", 2.4)), add(Shape::Line("M14,5.9V13", 2.4))];
const GLOBE: &[Part] = &[add(Shape::Fill(
    "M14,5.168C18.877,5.168 22.83,9.122 22.83,14C22.83,18.878 18.877,22.832 14,22.832C9.123,22.832 5.17,18.878 5.17,14\
     C5.17,9.122 9.123,5.168 14,5.168Z\
     M12.141,14.822C12.222,16.734 12.577,18.385 13.063,19.549C13.342,20.217 13.628,20.64 13.853,20.868C13.916,20.931 13.965,20.97 14,20.995\
     C14.035,20.97 14.084,20.931 14.147,20.868C14.372,20.64 14.658,20.217 14.937,19.549C15.423,18.385 15.778,16.734 15.859,14.822H12.141Z\
     M7.026,14.822C7.335,17.477 9.123,19.678 11.545,20.583C10.871,19.137 10.421,17.105 10.333,14.822H7.026Z\
     M17.667,14.822C17.579,17.106 17.128,19.137 16.454,20.583C18.876,19.679 20.665,17.477 20.974,14.822H17.667Z\
     M11.545,7.417C9.123,8.321 7.335,10.524 7.026,13.179H10.333C10.421,10.895 10.87,8.863 11.545,7.417Z\
     M14,7.006C13.965,7.03 13.916,7.07 13.853,7.133C13.628,7.362 13.342,7.784 13.063,8.452C12.577,9.617 12.222,11.268 12.141,13.179\
     H15.859C15.778,11.268 15.423,9.617 14.937,8.452C14.658,7.784 14.372,7.362 14.147,7.133C14.084,7.07 14.035,7.03 14,7.006Z\
     M16.454,7.417C17.128,8.863 17.579,10.895 17.667,13.179H20.974C20.665,10.524 18.876,8.321 16.454,7.417Z",
))];

const CLOCK: &[Part] = &[
    add(Shape::Fill("M14,5.3A8.7,8.7 0 1 1 14,22.7A8.7,8.7 0 1 1 14,5.3Z")),
    cut(Shape::Line("M14,8.9V14.3L17.4,16.4", 2.0)),
];

/// A calculator, drawn for Project Aether: the body, the screen across the
/// top of it and six keys under the screen.
const CALCULATOR: &[Part] = &[
    add(Shape::Fill("M9,3.5H19A3,3 0 0 1 22,6.5V21.5A3,3 0 0 1 19,24.5H9A3,3 0 0 1 6,21.5V6.5A3,3 0 0 1 9,3.5Z")),
    cut(Shape::EvenOdd(
        "M8.8,6.8H19.2V10.8H8.8Z\
         M8.8,12.8H12.4V15H8.8Z\
         M15.6,12.8H19.2V15H15.6Z\
         M8.8,16.2H12.4V18.4H8.8Z\
         M15.6,16.2H19.2V18.4H15.6Z\
         M8.8,19.6H12.4V21.8H8.8Z\
         M15.6,19.6H19.2V21.8H15.6Z",
    )),
];

const TERMINAL: &[Part] = &[
    add(Shape::Fill("M8.6,6.4H19.4A3.4,3.4 0 0 1 22.8,9.8V18.2A3.4,3.4 0 0 1 19.4,21.6H8.6A3.4,3.4 0 0 1 5.2,18.2V9.8A3.4,3.4 0 0 1 8.6,6.4Z")),
    cut(Shape::Line("M9.4,10.6L12.5,13.3L9.4,16", 1.9)),
    cut(Shape::Line("M14.4,16.4H18.6", 1.9)),
];

const USB: &[Part] = &[
    add(Shape::Fill("M11.4,4.6H16.6A1.1,1.1 0 0 1 17.7,5.7V9.1A1.1,1.1 0 0 1 16.6,10.2H11.4A1.1,1.1 0 0 1 10.3,9.1V5.7A1.1,1.1 0 0 1 11.4,4.6Z")),
    cut(Shape::Fill("M12.3,6.5H13A0.4,0.4 0 0 1 13.4,6.9V7.7A0.4,0.4 0 0 1 13,8.1H12.3A0.4,0.4 0 0 1 11.9,7.7V6.9A0.4,0.4 0 0 1 12.3,6.5Z")),
    cut(Shape::Fill("M15,6.5H15.7A0.4,0.4 0 0 1 16.1,6.9V7.7A0.4,0.4 0 0 1 15.7,8.1H15A0.4,0.4 0 0 1 14.6,7.7V6.9A0.4,0.4 0 0 1 15,6.5Z")),
    add(Shape::Fill("M11,9.4H17A2.6,2.6 0 0 1 19.6,12V15.7A2.6,2.6 0 0 1 17,18.3H11A2.6,2.6 0 0 1 8.4,15.7V12A2.6,2.6 0 0 1 11,9.4Z")),
    add(Shape::Line("M14,17.8V22.6", 2.4)),
];

const USB_DRIVE: &[Part] = &[
    add(Shape::Fill("M11.5,4.6H16.5A1,1 0 0 1 17.5,5.6V8.9A1,1 0 0 1 16.5,9.9H11.5A1,1 0 0 1 10.5,8.9V5.6A1,1 0 0 1 11.5,4.6Z")),
    cut(Shape::Fill(
        "M12.35,6.4H13.05A0.35,0.35 0 0 1 13.4,6.75V7.55A0.35,0.35 0 0 1 13.05,7.9H12.35A0.35,0.35 0 0 1 12,7.55V6.75A0.35,0.35 0 0 1 12.35,6.4Z",
    )),
    cut(Shape::Fill(
        "M14.95,6.4H15.65A0.35,0.35 0 0 1 16,6.75V7.55A0.35,0.35 0 0 1 15.65,7.9H14.95A0.35,0.35 0 0 1 14.6,7.55V6.75A0.35,0.35 0 0 1 14.95,6.4Z",
    )),
    add(Shape::Fill("M11.3,9.3H16.7A2.8,2.8 0 0 1 19.5,12.1V20.6A2.8,2.8 0 0 1 16.7,23.4H11.3A2.8,2.8 0 0 1 8.5,20.6V12.1A2.8,2.8 0 0 1 11.3,9.3Z")),
    cut(Shape::Fill("M14,17.05A1.25,1.25 0 1 1 14,19.55A1.25,1.25 0 1 1 14,17.05Z")),
];

const LAYERS: &[Part] = &[add(Shape::Fill(
    "M21.176,15.189C22.106,15.728 22.572,15.997 22.728,16.349C22.864,16.655 22.864,17.005 22.728,17.312C22.572,17.663 22.106,17.933 21.176,18.472\
     L16.215,21.341C15.409,21.808 15.005,22.042 14.576,22.133C14.196,22.214 13.804,22.214 13.425,22.133C12.995,22.041 12.592,21.808 11.785,21.341\
     L6.825,18.472C5.894,17.933 5.429,17.663 5.272,17.312C5.136,17.005 5.136,16.655 5.272,16.349C5.429,15.997 5.894,15.728 6.825,15.189\
     L7.819,14.613L11.547,16.771C11.586,16.794 11.625,16.816 11.663,16.838C12.239,17.172 12.732,17.458 13.276,17.574\
     C13.753,17.676 14.247,17.676 14.724,17.574C15.268,17.458 15.761,17.172 16.337,16.838C16.375,16.816 16.414,16.794 16.453,16.771\
     L20.18,14.613L21.176,15.189Z\
     M13.507,5.858C13.832,5.789 14.168,5.789 14.494,5.858C14.862,5.937 15.208,6.137 15.899,6.537L21.176,9.59C22.107,10.128 22.572,10.399 22.728,10.75\
     C22.864,11.057 22.865,11.406 22.728,11.713C22.572,12.064 22.106,12.333 21.176,12.872L15.899,15.925C15.208,16.325 14.861,16.525 14.494,16.604\
     C14.168,16.673 13.832,16.673 13.507,16.604C13.139,16.525 12.793,16.325 12.101,15.925L6.825,12.872C5.894,12.333 5.429,12.064 5.272,11.713\
     C5.136,11.406 5.136,11.057 5.272,10.75C5.428,10.399 5.894,10.128 6.825,9.59L12.101,6.537C12.793,6.137 13.139,5.937 13.507,5.858Z",
))];

const PHONE: &[Part] = &[add(Shape::Fill(
    "M15.595,4.326C17.435,4.326 18.355,4.326 19.058,4.684C19.676,4.999 20.179,5.502 20.494,6.12C20.852,6.823 20.852,7.743 20.852,9.583\
     V18.418C20.852,20.258 20.852,21.179 20.494,21.882C20.179,22.5 19.676,23.003 19.058,23.318C18.355,23.676 17.435,23.675 15.595,23.675\
     H12.405C10.565,23.675 9.645,23.676 8.942,23.318C8.324,23.003 7.821,22.5 7.506,21.882C7.148,21.179 7.148,20.258 7.148,18.418V9.583\
     C7.148,7.743 7.148,6.823 7.506,6.12C7.821,5.502 8.324,4.999 8.942,4.684C9.645,4.326 10.565,4.326 12.405,4.326H15.595Z\
     M11.421,5.967C10.501,5.967 10.04,5.966 9.689,6.146C9.38,6.303 9.129,6.555 8.971,6.864C8.792,7.216 8.792,7.676 8.792,8.595V19.406\
     C8.792,20.325 8.792,20.785 8.971,21.137C9.129,21.446 9.38,21.698 9.689,21.855C10.04,22.034 10.501,22.034 11.421,22.034H16.579\
     C17.499,22.034 17.959,22.034 18.311,21.855C18.62,21.698 18.871,21.446 19.029,21.137C19.208,20.785 19.208,20.325 19.208,19.406V8.595\
     C19.208,7.676 19.208,7.216 19.029,6.864C18.871,6.555 18.62,6.303 18.311,6.146C17.959,5.967 17.499,5.967 16.579,5.967H11.421Z\
     M16.875,7.117C17.289,7.117 17.496,7.117 17.654,7.197C17.793,7.268 17.906,7.381 17.977,7.52C18.058,7.679 18.058,7.886 18.058,8.3V19.701\
     C18.058,20.115 18.058,20.322 17.977,20.48C17.906,20.619 17.793,20.733 17.654,20.804C17.496,20.884 17.289,20.884 16.875,20.884H11.125\
     C10.711,20.884 10.504,20.884 10.346,20.804C10.207,20.733 10.094,20.619 10.023,20.48C9.942,20.322 9.943,20.115 9.943,19.701V8.3\
     C9.943,7.886 9.942,7.679 10.023,7.52C10.094,7.381 10.207,7.268 10.346,7.197C10.504,7.117 10.711,7.117 11.125,7.117H16.875Z",
))];

const LOCK: &[Part] = &[add(Shape::EvenOdd(
    "M11.079,11.281C11.222,11.281 11.371,11.281 11.529,11.281H16.471C16.628,11.281 16.778,11.281 16.919,11.281V9.205\
     C16.919,7.593 15.612,6.285 13.999,6.285C12.387,6.285 11.079,7.593 11.079,9.205V11.281Z\
     M9.415,11.355C9.124,11.399 8.886,11.467 8.672,11.576C8.162,11.836 7.747,12.251 7.487,12.761C7.192,13.341 7.192,14.1 7.192,15.618V18.355\
     C7.192,19.873 7.192,20.632 7.487,21.212C7.747,21.722 8.162,22.137 8.672,22.396C9.252,22.692 10.011,22.692 11.529,22.692H16.471\
     C17.989,22.692 18.748,22.692 19.328,22.396C19.838,22.137 20.253,21.722 20.513,21.212C20.808,20.632 20.808,19.873 20.808,18.355V15.618\
     C20.808,14.1 20.808,13.341 20.513,12.761C20.253,12.251 19.838,11.836 19.328,11.576C19.114,11.467 18.876,11.399 18.583,11.355V9.205\
     C18.583,6.673 16.531,4.621 13.999,4.621C11.468,4.621 9.415,6.673 9.415,9.205V11.355Z",
))];

const MORE: &[Part] = &[add(Shape::EvenOdd(
    "M14,22.627C18.764,22.627 22.626,18.765 22.626,14.001C22.626,9.237 18.764,5.375 14,5.375C9.236,5.375 5.374,9.237 5.374,14.001\
     C5.374,18.765 9.236,22.627 14,22.627Z\
     M10.304,15.028C10.871,15.028 11.331,14.569 11.331,14.002C11.331,13.434 10.871,12.975 10.304,12.975C9.737,12.975 9.278,13.434 9.278,14.002\
     C9.278,14.569 9.737,15.028 10.304,15.028Z\
     M15.028,14.002C15.028,14.569 14.568,15.028 14.001,15.028C13.434,15.028 12.974,14.569 12.974,14.002C12.974,13.434 13.434,12.975 14.001,12.975\
     C14.568,12.975 15.028,13.434 15.028,14.002Z\
     M17.697,15.028C18.264,15.028 18.724,14.569 18.724,14.002C18.724,13.434 18.264,12.975 17.697,12.975C17.13,12.975 16.67,13.434 16.67,14.002\
     C16.67,14.569 17.13,15.028 17.697,15.028Z",
))];

const FOLDER: &[Part] = &[add(Shape::Fill(
    "M24.478,12.406V17.461C24.478,19.421 24.479,20.402 24.097,21.151C23.761,21.809 23.225,22.345 22.567,22.68C21.818,23.062 20.838,23.062 18.878,23.062\
     H9.121C7.162,23.062 6.182,23.062 5.433,22.68C4.774,22.345 4.239,21.809 3.904,21.151C3.522,20.402 3.522,19.422 3.522,17.461V11.982\
     H24.476C24.476,12.118 24.478,12.259 24.478,12.406Z\
     M10.363,4.938C10.793,4.938 11.009,4.938 11.213,4.975C11.541,5.035 11.852,5.165 12.124,5.357C12.294,5.477 12.446,5.63 12.748,5.937\
     C13.01,6.204 13.142,6.338 13.289,6.442C13.526,6.608 13.797,6.721 14.081,6.773C14.259,6.805 14.446,6.806 14.821,6.806H18.878\
     C20.838,6.806 21.818,6.806 22.567,7.187C23.225,7.523 23.761,8.059 24.097,8.717C24.336,9.187 24.425,9.749 24.458,10.586\
     C24.389,10.585 24.31,10.583 24.218,10.583H3.522V10.538C3.522,8.578 3.522,7.598 3.904,6.849C4.239,6.191 4.774,5.655 5.433,5.32\
     C6.182,4.938 7.162,4.938 9.121,4.938H10.363Z",
))];

/// The stock folder is drawn without a tile, filling 21 units of its box
/// across; on a tile it is drawn as wide as the widest stock glyphs.
const FOLDER_SCALE: f32 = 18.6 / 21.0;

const LOG: &[Part] = &[
    add(Shape::Fill("M10.3,4.8H17.7A3,3 0 0 1 20.7,7.8V20.2A3,3 0 0 1 17.7,23.2H10.3A3,3 0 0 1 7.3,20.2V7.8A3,3 0 0 1 10.3,4.8Z")),
    cut(Shape::Line("M10.6,10H17.4", 1.8)),
    cut(Shape::Line("M10.6,13.9H17.4", 1.8)),
    cut(Shape::Line("M10.6,17.8H14.6", 1.8)),
];

const SHIELD: &[Part] = &[add(Shape::EvenOdd(
    "M14.27,5.155C14.471,5.189 14.665,5.274 15.053,5.443L19.325,7.312C20.214,7.7 20.658,7.895 20.981,8.212C21.264,8.489 21.481,8.841 21.602,9.217\
     C21.741,9.648 21.716,10.125 21.666,11.08C21.587,12.58 21.477,14.392 21.352,15.635C21.117,17.962 18.831,20.601 14.838,22.604\
     C14.555,22.745 14.413,22.816 14.237,22.85C14.094,22.876 13.906,22.876 13.763,22.85C13.586,22.816 13.445,22.745 13.162,22.604\
     C9.168,20.601 6.882,17.962 6.648,15.635C6.522,14.392 6.413,12.58 6.334,11.08C6.284,10.125 6.259,9.648 6.398,9.217\
     C6.519,8.841 6.736,8.489 7.018,8.212C7.342,7.895 7.786,7.7 8.675,7.312L12.946,5.443C13.335,5.274 13.529,5.189 13.73,5.155\
     C13.909,5.125 14.091,5.125 14.27,5.155Z\
     M17.187,10.977L13.27,14.894L11.448,13.072C11.34,12.964 11.286,12.909 11.228,12.88C11.111,12.82 10.972,12.82 10.855,12.88\
     C10.797,12.909 10.743,12.964 10.635,13.072L10.635,13.072C10.527,13.18 10.473,13.234 10.443,13.292C10.384,13.409 10.384,13.547 10.443,13.665\
     C10.473,13.723 10.527,13.777 10.635,13.885L12.864,16.114C12.972,16.222 13.026,16.276 13.084,16.306C13.202,16.365 13.34,16.365 13.457,16.306\
     C13.515,16.276 13.569,16.222 13.677,16.114L13.677,16.114L13.677,16.114C13.714,16.077 13.745,16.047 13.77,16.02L18,11.791\
     C18.108,11.682 18.162,11.628 18.191,11.571C18.251,11.453 18.251,11.315 18.191,11.198C18.162,11.14 18.108,11.086 18,10.977\
     L18,10.977C17.892,10.869 17.838,10.815 17.78,10.786C17.663,10.726 17.524,10.726 17.407,10.786C17.349,10.815 17.295,10.869 17.187,10.977Z",
))];

const BATTERY: &[Part] = &[add(Shape::EvenOdd(
    "M5.278,10.103C4.964,10.718 4.964,11.523 4.964,13.133V14.868C4.964,16.478 4.964,17.283 5.278,17.898C5.553,18.439 5.993,18.879 6.534,19.155\
     C7.149,19.468 7.954,19.468 9.564,19.468H16.101C17.712,19.468 18.517,19.468 19.132,19.155C19.673,18.879 20.112,18.439 20.388,17.898\
     C20.701,17.283 20.701,16.478 20.701,14.868V13.133C20.701,11.523 20.701,10.718 20.388,10.103C20.112,9.562 19.673,9.122 19.132,8.847\
     C18.517,8.533 17.712,8.533 16.101,8.533H9.564C7.954,8.533 7.149,8.533 6.534,8.847C5.993,9.122 5.553,9.562 5.278,10.103Z\
     M21.571,11.851C21.524,11.956 21.524,12.09 21.524,12.358V15.644C21.524,15.912 21.524,16.046 21.571,16.15C21.624,16.269 21.719,16.364 21.838,16.418\
     C21.943,16.465 22.077,16.465 22.345,16.465C22.613,16.465 22.747,16.465 22.852,16.418C22.971,16.364 23.066,16.269 23.119,16.15\
     C23.166,16.046 23.166,15.912 23.166,15.644V12.358C23.166,12.09 23.166,11.956 23.119,11.851C23.066,11.732 22.971,11.637 22.852,11.583\
     C22.747,11.536 22.613,11.536 22.345,11.536C22.077,11.536 21.943,11.536 21.838,11.583C21.719,11.637 21.624,11.732 21.571,11.851Z",
))];

const SYSTEMS: &[Part] = &[
    add(Shape::Fill("M8.6,5.2H15A2.4,2.4 0 0 1 17.4,7.6V17.2A2.4,2.4 0 0 1 15,19.6H8.6A2.4,2.4 0 0 1 6.2,17.2V7.6A2.4,2.4 0 0 1 8.6,5.2Z")),
    // The gap that sets the phone in front apart from the one behind.
    cut(Shape::Fill("M13,7.1H19.4A3.7,3.7 0 0 1 23.1,10.8V20.4A3.7,3.7 0 0 1 19.4,24.1H13A3.7,3.7 0 0 1 9.3,20.4V10.8A3.7,3.7 0 0 1 13,7.1Z")),
    add(Shape::Fill("M13,8.4H19.4A2.4,2.4 0 0 1 21.8,10.8V20.4A2.4,2.4 0 0 1 19.4,22.8H13A2.4,2.4 0 0 1 10.6,20.4V10.8A2.4,2.4 0 0 1 13,8.4Z")),
    cut(Shape::Line("M14.4,20.2H18", 1.3)),
];

/// A disc: its face, the ring at its centre and the two arcs its surface
/// throws the light back with, and the dot the ring goes round.
///
/// The whole of it is cut with the odd-even rule, the user's own drawing
/// being written that way: the arcs and the ring lie inside the face once
/// over it and so are left out, and the dot lies inside the ring as well,
/// so it fills back in where the ring had a hole.
const MUSIC: &[Part] = &[add(Shape::EvenOdd(
    "M14,5C9.043,5,5,9.043,5,14.007C5,18.965,9.043,23,14,23C18.965,23,23,18.965,23,14.007\
     C23,9.043,18.965,5,14,5ZM9.26,14.007C9.26,14.42,8.93,14.757,8.51,14.758\
     C8.098,14.758,7.76,14.42,7.76,14.007C7.76,10.565,10.558,7.76,14,7.76\
     C14.412,7.76,14.75,8.098,14.75,8.51C14.75,8.922,14.412,9.26,14,9.26\
     C11.39,9.26,9.26,11.39,9.26,14.007ZM11.622,14A2.378,2.378 0 0 1 14,11.622\
     C15.32,11.622,16.385,12.687,16.385,14C16.385,15.313,15.32,16.385,14,16.385\
     A2.385,2.385 0 0 1 11.622,14ZM14,20.24C13.588,20.24,13.25,19.902,13.25,19.49\
     C13.25,19.078,13.588,18.74,14,18.74C16.617,18.74,18.748,16.618,18.748,14.007\
     C18.748,13.587,19.078,13.258,19.497,13.258C19.91,13.258,20.247,13.588,20.248,14.007\
     C20.248,17.443,17.442,20.24,14,20.24ZM14.885,14C14.885,13.512,14.487,13.123,14,13.123\
     C13.52,13.123,13.123,13.512,13.123,14C13.123,14.488,13.52,14.885,14,14.885\
     C14.488,14.885,14.885,14.487,14.885,14Z",
))];

/// A globe: its ring, a line across it and a line round it.
///
/// All three are drawn at one weight, so the globe reads as a single
/// drawing rather than as a heavy ring with thin marks inside it. A filled
/// ring was too heavy against the two lines it sits around.
const BROWSER: &[Part] = &[
    // The ring.
    add(Shape::Line("M14,5.2A8.8,8.8 0 1 1 14,22.8A8.8,8.8 0 1 1 14,5.2Z", 1.7)),
    // The line across it, and the line round it.
    add(Shape::Line("M5.2,14H22.8", 1.7)),
    add(Shape::Line("M14,5.2A4.4,8.8 0 0 1 14,22.8A4.4,8.8 0 0 1 14,5.2", 1.7)),
];

/// A monitor on its foot: the screen, the neck under it and the stand, the
/// screen's glass cut out of the frame. The drawing is the user's own
/// `taskmgr.svg`, laid in this box at 19 units across: the drawing fills its
/// own box edge to edge, so it is set a little smaller than the glyphs that
/// keep a margin and reach the same size on a tile.
///
/// Both of its subpaths are written as one outline cut by the odd-even rule:
/// the frame and the screen inside it are one path each, and where the two
/// go round together the screen is left out.
const TASKS: &[Part] = &[add(Shape::EvenOdd(
    "M20.909,5.364L7.091,5.364A2.591,2.591 0 0 0 4.5,7.955L4.5,16.591\
     A2.591,2.591 0 0 0 7.091,19.182L13.136,19.182L13.136,20.909L10.545,20.909\
     A0.864,0.864 0 0 0 10.545,22.636L17.455,22.636A0.864,0.864 0 0 0 17.455,20.909L14.864,20.909\
     L14.864,19.182L20.909,19.182A2.591,2.591 0 0 0 23.5,16.591L23.5,7.955\
     A2.591,2.591 0 0 0 20.909,5.364ZM21.773,16.591A0.864,0.864 0 0 1 20.909,17.455L7.091,17.455\
     A0.864,0.864 0 0 1 6.227,16.591L6.227,7.955A0.864,0.864 0 0 1 7.091,7.091L20.909,7.091\
     A0.864,0.864 0 0 1 21.773,7.955L21.773,16.591Z",
))];

/// A sun behind a cloud: what a weather page shows. Drawn the stock way
/// rather than copied, since the stock Settings has no weather of its own:
/// the sun's disc and the four rays it throws, then the cloud in front of
/// it built from rounds and a flat foot.
const WEATHER: &[Part] = &[
    // The sun's disc, and the four rays clear of the cloud.
    add(Shape::Fill("M11.2,8.5A3.3,3.3 0 1 1 11.2,15.1A3.3,3.3 0 1 1 11.2,8.5Z")),
    add(Shape::Line("M11.2,5.0V6.8", 1.5)),
    add(Shape::Line("M11.2,16.8V18.6", 1.5)),
    add(Shape::Line("M5.3,11.8H7.1", 1.5)),
    add(Shape::Line("M15.3,11.8H17.1", 1.5)),
    // The cloud's three rounds and the foot they stand on, filled one over
    // the other: the union is the cloud.
    add(Shape::Fill("M11.6,17.5A2.9,2.9 0 1 1 11.6,23.3A2.9,2.9 0 1 1 11.6,17.5Z")),
    add(Shape::Fill("M16.6,15.3A3.7,3.7 0 1 1 16.6,22.7A3.7,3.7 0 1 1 16.6,15.3Z")),
    add(Shape::Fill("M21.0,18.6A2.5,2.5 0 1 1 21.0,23.6A2.5,2.5 0 1 1 21.0,18.6Z")),
    add(Shape::Fill("M11.5,20.6H21.0V23.8H11.5Z")),
];

/// The stopwatch's four keys, as the watch face's own drawing tool drew
/// them — the same 1024-unit box, scaled into this file's 28.
///
/// The three squareish ones were drawn as one set and sit at different
/// heights in their box — the flag fills it, the triangle a little over
/// half — so each has a scale of its own that lifts it until all three
/// reach the same top and bottom. That shared height is what makes three
/// pictures by two hands read as one set of keys.
///
/// `replay` is the drawing as it came, arrow to the left, and it is wide
/// rather than tall: its ring is nearly four times as wide as it is high,
/// so it is set to the box's width and stands on the box's middle, 28 by
/// 12.33. Lifted to the box's height instead it would run out of the box
/// on both sides and be cut off there.
const PLAY: &[Part] = &[add(Shape::Fill("M9.352,5.852L22.149,14L9.352,22.148L9.352,5.851Z"))];

const PAUSE: &[Part] = &[add(Shape::Fill(
    "M16.352,5.852L21,5.852L21,22.149L16.352,22.149L16.352,5.852ZM7,22.148L7,5.851L11.648,5.851\
     L11.648,22.148L7,22.148Z",
))];

const FLAG: &[Part] = &[add(Shape::Fill(
    "M3.5,2.333L13.316,2.333L15.65,4.667L24.999,4.667L22.241,12.25L24.999,19.833L14.684,19.833\
     L12.35,17.5L5.833,17.5L5.833,26.25L3.5,26.25L3.5,2.333ZM5.833,15.167L13.316,15.167\
     L15.65,17.5L21.667,17.5L19.759,12.25L21.667,7L14.684,7L12.35,4.667L5.833,4.667L5.833,15.167Z",
))];

/// The reset key's picture, redrawn from the user's own `reset.svg`: a ring
/// broken at the left with an arrow turning back into it. It is laid out at
/// the box's full width, so it needs no scale of its own; see the note
/// over [`PLAY`] for why it is the one key not lifted to the box's height.
const REPLAY: &[Part] = &[add(Shape::Fill(
    "M14.386,9.248C17.511,9.248,20.316,10.169,22.799,12.01\
     C25.283,13.852,27.017,16.228,28.001,19.138L24.79,20.166\
     C24.062,17.939,22.745,16.13,20.84,14.739C18.935,13.348,16.783,12.653,14.386,12.653\
     C11.732,12.653,9.377,13.509,7.322,15.222L12.331,20.166L0.001,20.166L0.001,7.836L4.882,12.78\
     C7.579,10.426,10.747,9.248,14.387,9.248Z",
))];

/// The height each of the three squareish pictures has in the drawings'
/// 1024 box — the scale that lifts it to the whole box.
const PLAY_SCALE: f32 = 1024.0 / 596.0;
const PAUSE_SCALE: f32 = 1024.0 / 596.0;
const FLAG_SCALE: f32 = 1024.0 / 874.666_7;

/// The timer's two keys, which take a minute off the length and put one on:
/// a bar, and two bars crossed. They are set in the same box as the
/// stopwatch's pictures but kept at their own size — a bar stretched to fill
/// the box the way a triangle does would read as a slab rather than a minus
/// — and they are line strokes rather than outlines, so their ends are round
/// at whatever size they are drawn. Their weight is the stopwatch's pause
/// bar's, which is what keeps a cross from reading thinner than the picture
/// beside it.
const MINUS: &[Part] = &[add(Shape::Line("M5,14H23", 6.0))];

const PLUS: &[Part] = &[add(Shape::Line("M5,14H23M14,5V23", 6.0))];

/// What `icon`'s glyph is drawn from, and at what share of its size.
fn outline(icon: Icon) -> (&'static [Part], f32) {
    match icon {
        Icon::Install => (INSTALL, 1.0),
        Icon::Backup => (BACKUP, 1.0),
        Icon::Restore => (RESTORE, 1.0),
        Icon::Mount => (MOUNT, 1.0),
        Icon::Wipe => (WIPE, 1.0),
        Icon::Gear => (GEAR, 1.0),
        Icon::Power => (POWER, 1.0),
        Icon::Globe => (GLOBE, 1.0),
        Icon::Clock => (CLOCK, 1.0),
        Icon::Calculator => (CALCULATOR, 1.0),
        Icon::Terminal => (TERMINAL, 1.0),
        Icon::Usb => (USB, 1.0),
        Icon::UsbDrive => (USB_DRIVE, 1.0),
        Icon::Layers => (LAYERS, 1.0),
        Icon::Phone => (PHONE, 1.0),
        Icon::Lock => (LOCK, 1.0),
        Icon::More => (MORE, 1.0),
        Icon::Folder => (FOLDER, FOLDER_SCALE),
        Icon::Log => (LOG, 1.0),
        Icon::Shield => (SHIELD, 1.0),
        Icon::Wlan => (WLAN, 1.0),
        Icon::Battery => (BATTERY, 1.0),
        Icon::Systems => (SYSTEMS, 1.0),
        Icon::Music => (MUSIC, 1.0),
        Icon::Browser => (BROWSER, 1.0),
        Icon::Tasks => (TASKS, 1.0),
        Icon::Weather => (WEATHER, 1.0),
        Icon::Play => (PLAY, PLAY_SCALE),
        Icon::Pause => (PAUSE, PAUSE_SCALE),
        Icon::Flag => (FLAG, FLAG_SCALE),
        Icon::Replay => (REPLAY, 1.0),
        Icon::Minus => (MINUS, 1.0),
        Icon::Plus => (PLUS, 1.0),
        Icon::Settings => (SETTINGS, 1.0),
    }
}

/// Draw `icon`'s glyph in `ink`, filling a box `size` across centred on
/// (`cx`, `cy`) as it fills its tile.
pub fn glyph(c: &mut Canvas, icon: Icon, cx: f32, cy: f32, size: f32, ink: Rgba) {
    draw(c, Art::Glyph(icon), cx, cy, size, ink);
}

/// Draw `icon` as a row shows it: its tile in `color`, the tile's top-left
/// corner at (`x`, `y`), and its glyph in white over it.
pub fn tile(c: &mut Canvas, t: &Theme, x: f32, y: f32, color: Rgba, icon: Icon) {
    let s = t.m.icon;
    let (cx, cy) = (x + s / 2.0, y + s / 2.0);
    draw(c, Art::Tile, cx, cy, s, color);
    glyph(c, icon, cx, cy, s, Rgba::rgb(255, 255, 255));
}

/// The stock tile on its own, `size` across with its top-left corner at
/// (`x`, `y`), shading from `top` to `bottom` down it: the tile a row's
/// icon sits on, laid as a desktop tile the light falls on. It is the same
/// outline either way, so a desktop tile and a row's tile read as one
/// shape rather than as two takes on a rounded square.
pub fn tile_graded(c: &mut Canvas, x: f32, y: f32, size: f32, top: Rgba, bottom: Rgba) {
    let (left, up) = (x.round() as i32, y.round() as i32);
    let side = size.ceil() as i32;
    let (rows, cols) = (c.held(), c.cols());
    let (y0, y1) = (up.max(rows.start as i32), (up + side).min(rows.end as i32));
    let (x0, x1) = (left.max(cols.start as i32), (left + side).min(cols.end as i32));
    if y0 >= y1 || x0 >= x1 {
        return;
    }
    let cover = coverage(Art::Tile, size);
    for y in y0..y1 {
        let ink = mix(top, bottom, ((y - up) as f32 + 0.5) / size);
        for x in x0..x1 {
            let a = cover.alpha[((y - up) as u32 * cover.side + (x - left) as u32) as usize];
            if a > 0 {
                c.blend(x, y, ink, a as f32 / 255.0);
            }
        }
    }
}

/// What is drawn from outlines: a tile, or a glyph.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Art {
    Tile,
    Glyph(Icon),
}

/// What a drawing covers of each pixel of its square box, from the top-left
/// one, row by row.
struct Coverage {
    side: u32,
    alpha: Vec<u8>,
}

/// A drawing as it was made: what, at what size, and what it covers.
struct Made {
    art: Art,
    size: f32,
    cover: Arc<Coverage>,
}

/// Each drawing made, which the pages ask for over and over, the same few
/// at the same few sizes.
static DRAWN: Mutex<Vec<Made>> = Mutex::new(Vec::new());

/// Draw `art` in `ink`, its box `size` across centred on (`cx`, `cy`) and
/// set on whole pixels, as the stock views are laid out; only what falls in
/// the part of the canvas held is drawn, and nothing at all is made when
/// none of it does.
fn draw(c: &mut Canvas, art: Art, cx: f32, cy: f32, size: f32, ink: Rgba) {
    let (left, top) = ((cx - size / 2.0).round() as i32, (cy - size / 2.0).round() as i32);
    let side = size.ceil() as i32;
    let (rows, cols) = (c.held(), c.cols());
    let (y0, y1) = (top.max(rows.start as i32), (top + side).min(rows.end as i32));
    let (x0, x1) = (left.max(cols.start as i32), (left + side).min(cols.end as i32));
    if y0 >= y1 || x0 >= x1 {
        return;
    }
    let cover = coverage(art, size);
    for y in y0..y1 {
        for x in x0..x1 {
            let a = cover.alpha[((y - top) as u32 * cover.side + (x - left) as u32) as usize];
            if a > 0 {
                c.blend(x, y, ink, a as f32 / 255.0);
            }
        }
    }
}

/// What `art` covers of its box `size` across, made the first time it is
/// asked for at that size.
fn coverage(art: Art, size: f32) -> Arc<Coverage> {
    let found = DRAWN.lock().expect("no one panics holding the drawings").iter().find(|m| m.art == art && m.size == size).map(|m| m.cover.clone());
    if let Some(cover) = found {
        return cover;
    }
    let (parts, scale) = match art {
        Art::Tile => (TILE, 1.0),
        Art::Glyph(icon) => outline(icon),
    };
    let cover = Arc::new(rasterize(parts, scale, size));
    DRAWN.lock().expect("no one panics holding the drawings").push(Made { art, size, cover: cover.clone() });
    cover
}

/// What `parts` cover of a box `size` across, at `scale` of their size
/// about the middle of the box they are laid out in. They are drawn
/// together, so that what one part cuts out of another shows what lies
/// under the whole.
fn rasterize(parts: &[Part], scale: f32, size: f32) -> Coverage {
    let side = size.ceil() as u32;
    let mut art = Pixmap::new(side, side).expect("a glyph's box holds pixels");
    let k = size / BOX * scale;
    let at = Transform::from_translate(size / 2.0, size / 2.0).pre_scale(k, k).pre_translate(-BOX / 2.0, -BOX / 2.0);
    for part in parts {
        let mut paint = Paint::default();
        paint.set_color(Color::WHITE);
        paint.blend_mode = if part.cut { BlendMode::Clear } else { BlendMode::SourceOver };
        match part.shape {
            Shape::Fill(d) => art.fill_path(&path(d), &paint, FillRule::Winding, at, None),
            Shape::EvenOdd(d) => art.fill_path(&path(d), &paint, FillRule::EvenOdd, at, None),
            Shape::Line(d, width) => {
                let stroke = Stroke { width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
                art.stroke_path(&path(d), &paint, &stroke, at, None);
            }
        }
    }
    Coverage { side, alpha: art.pixels().iter().map(|px| px.alpha()).collect() }
}

/// The outline SVG path data `d` describes.
fn path(d: &str) -> tiny_skia::Path {
    let mut b = PathBuilder::new();
    for segment in SimplifyingPathParser::from(d) {
        match segment.unwrap_or_else(|e| panic!("an icon's outline {d:?} does not read: {e}")) {
            SimplePathSegment::MoveTo { x, y } => b.move_to(x as f32, y as f32),
            SimplePathSegment::LineTo { x, y } => b.line_to(x as f32, y as f32),
            SimplePathSegment::Quadratic { x1, y1, x, y } => b.quad_to(x1 as f32, y1 as f32, x as f32, y as f32),
            SimplePathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                b.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32)
            }
            SimplePathSegment::ClosePath => b.close(),
        }
    }
    b.finish().unwrap_or_else(|| panic!("an icon's outline {d:?} encloses nothing"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Icon; 28] = [
        Icon::Install,
        Icon::Backup,
        Icon::Restore,
        Icon::Mount,
        Icon::Wipe,
        Icon::Gear,
        Icon::Power,
        Icon::Globe,
        Icon::Clock,
        Icon::Calculator,
        Icon::Terminal,
        Icon::Usb,
        Icon::UsbDrive,
        Icon::Layers,
        Icon::Phone,
        Icon::Lock,
        Icon::More,
        Icon::Folder,
        Icon::Log,
        Icon::Shield,
        Icon::Wlan,
        Icon::Battery,
        Icon::Systems,
        Icon::Music,
        Icon::Browser,
        Icon::Tasks,
        Icon::Weather,
        Icon::Settings,
    ];

    /// Pixels a unit of the box takes in these tests.
    const UNIT: f32 = 10.0;

    /// `icon`'s glyph in white over black, its box 28 units across at
    /// `UNIT` pixels a unit.
    fn drawn(icon: Icon) -> Canvas {
        let side = (BOX * UNIT) as u32;
        let mut c = Canvas::new(side, side);
        c.clear(Rgba::rgb(0, 0, 0));
        glyph(&mut c, icon, side as f32 / 2.0, side as f32 / 2.0, side as f32, Rgba::rgb(255, 255, 255));
        c
    }

    /// Whether the glyph covers the point (`x`, `y`) of its box, in units.
    fn covers(c: &Canvas, x: f32, y: f32) -> bool {
        c.data[((((y * UNIT) as u32) * c.w + (x * UNIT) as u32) * 4) as usize] > 128
    }

    /// The box round what the glyph covers, in units: left, top, right,
    /// bottom.
    fn ink(c: &Canvas) -> (f32, f32, f32, f32) {
        let mut b = (f32::MAX, f32::MAX, 0.0f32, 0.0f32);
        for (i, px) in c.data.as_chunks::<4>().0.iter().enumerate() {
            if px[0] > 0 {
                let (x, y) = ((i as u32 % c.w) as f32, (i as u32 / c.w) as f32);
                b = (b.0.min(x), b.1.min(y), b.2.max(x + 1.0), b.3.max(y + 1.0));
            }
        }
        (b.0 / UNIT, b.1 / UNIT, b.2 / UNIT, b.3 / UNIT)
    }

    #[test]
    fn every_glyph_reads_and_stands_in_the_middle_of_its_tile_clear_of_its_edge() {
        for icon in ALL {
            let (l, t, r, b) = ink(&drawn(icon));
            assert!(r > l && b > t, "{icon:?} covers nothing");
            // As the stock glyphs keep within the tile's middle 23 units,
            // and stand within three quarters of one of its middle.
            assert!(l >= 2.5 && t >= 2.5 && r <= 25.5 && b <= 25.5, "{icon:?} reaches {l}, {t} to {r}, {b}");
            let (x, y) = ((l + r) / 2.0, (t + b) / 2.0);
            assert!((x - 14.0).abs() <= 0.75 && (y - 14.0).abs() <= 0.75, "{icon:?} stands at {x}, {y}");
        }
    }

    #[test]
    fn the_stopwatch_keys_are_laid_out_as_one_set_and_the_reset_arrow_keeps_its_own_shape() {
        // These are not on tiles, so the rule above — the stock glyphs'
        // middle 23 units — is not theirs to keep: a key's picture is meant to
        // fill the box it is given. What they must share is where they end up.
        // The three squareish ones were drawn as one set and sit at different
        // heights in their 1024 box, so each has a scale of its own; a picture
        // left taller than the one beside it would read as a heavier key
        // rather than a bigger one.
        let keys = [Icon::Play, Icon::Pause, Icon::Flag];
        let heights = keys.map(|icon| {
            let (l, t, r, b) = ink(&drawn(icon));
            assert!(r > l && b > t, "{icon:?} covers nothing");
            assert!(l >= 0.0 && t >= 0.0 && r <= BOX && b <= BOX, "{icon:?} runs past its box: {l}, {t} to {r}, {b}");
            b - t
        });
        for (icon, h) in keys.into_iter().zip(&heights) {
            assert!((h - heights[0]).abs() <= 0.5, "{icon:?} stands {h} tall, the play key {}", heights[0]);
        }
        // The reset arrow is the fourth key and the one picture here that is
        // wider than it is tall — nearly four times, as it was drawn. Lifted
        // to the others' height it would run past the box on both sides and be
        // cut off, so it is held to the box's width and to its middle instead.
        let (l, t, r, b) = ink(&drawn(Icon::Replay));
        assert!(r > l && b > t, "the reset arrow covers nothing");
        assert!(l >= 0.0 && t >= 0.0 && r <= BOX && b <= BOX, "the reset arrow runs past its box: {l}, {t} to {r}, {b}");
        assert!((r - l - BOX).abs() <= 0.5, "the reset arrow spans {} wide, not the box's {BOX}", r - l);
        let (x, y) = ((l + r) / 2.0, (t + b) / 2.0);
        assert!((x - 14.0).abs() <= 0.75 && (y - 14.0).abs() <= 0.75, "the reset arrow stands at {x}, {y}");
        assert!(b - t < heights[0] / 2.0, "the reset arrow stands {} tall: it is drawn wide, not square", b - t);
    }

    #[test]
    fn details_are_cut_out_of_the_glyphs_by_cutting_parts_and_by_the_even_odd_rule() {
        let clock = drawn(Icon::Clock);
        assert!(covers(&clock, 14.0, 6.6), "the face");
        assert!(!covers(&clock, 14.0, 11.5), "the minute hand cut out of it");
        let more = drawn(Icon::More);
        assert!(covers(&more, 14.0, 8.0), "the circle");
        assert!(!covers(&more, 14.0, 14.0), "the middle dot left out of it");
        let shield = drawn(Icon::Shield);
        assert!(covers(&shield, 14.0, 7.0), "the shield");
        assert!(!covers(&shield, 13.3, 15.4), "the tick left out of it");
    }

    #[test]
    fn a_tile_is_the_stock_shape_its_glyph_white_over_it() {
        let t = Theme::light(1200, 2608, 3.0);
        let s = t.m.icon;
        let side = s.ceil() as u32 + 2;
        let mut c = Canvas::new(side, side);
        let black = Rgba::rgb(0, 0, 0);
        c.clear(black);
        tile(&mut c, &t, 1.0, 1.0, Rgba::rgb(0, 0, 255), Icon::More);
        let px = |x: f32, y: f32| -> [u8; 4] { c.data[(((y as u32) * c.w + x as u32) * 4) as usize..][..4].try_into().unwrap() };
        assert_eq!(px(1.0, 1.0), [0, 0, 0, 255], "its corners are clear");
        assert_eq!(px(1.0 + s / 2.0, 2.0)[2], 255, "the middle of its edge is the tile's");
        assert_eq!(px(1.0 + s / 2.0, 1.0 + s * 8.0 / BOX), [255, 255, 255, 255], "the glyph is white over it");
        assert_eq!(px(1.0 + s / 2.0, 1.0 + s / 2.0)[2], 255, "and the tile shows through what is cut out of the glyph");
    }
}
