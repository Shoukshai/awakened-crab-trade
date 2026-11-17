//! Price-checker column beside PoE's inventory; no desktop API dependency.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
pub fn beside_inventory(output: Rect, game: Rect, width: i32) -> Rect {
    let width = width.clamp(1, output.width.max(1));
    let sidebar = (f64::from(game.height) * 370. / 600.).round() as i32;
    Rect {
        x: (game.x + game.width - sidebar - width).clamp(output.x, output.x + output.width - width),
        y: output.y,
        width,
        height: output.height,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_inset_game_inventory_edge_and_full_output_height() {
        let output = Rect {
            x: 0,
            y: 0,
            width: 2560,
            height: 1600,
        };
        let game = Rect {
            x: 13,
            y: 12,
            width: 2534,
            height: 1540,
        };
        let p = beside_inventory(output, game, 480);
        assert_eq!(
            p,
            Rect {
                x: 1117,
                y: 0,
                width: 480,
                height: 1600
            }
        );
        assert_eq!(p.x + p.width, 1597);
    }
    #[test]
    fn scaled_logical_output_and_negative_monitor_origins_stay_inside_output() {
        for output in [
            Rect {
                x: 0,
                y: 0,
                width: 1280,
                height: 800,
            },
            Rect {
                x: -1920,
                y: 20,
                width: 1920,
                height: 1080,
            },
        ] {
            for width in [360, 480, 1200, 4000] {
                let p = beside_inventory(output, output, width);
                assert!(p.x >= output.x && p.x + p.width <= output.x + output.width);
                assert_eq!((p.y, p.height), (output.y, output.height));
            }
        }
    }
}
