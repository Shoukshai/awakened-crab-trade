//! Blank success is a failed capture too; try each independent backend once.
pub const BACKENDS: [&str; 4] = [
    "Windows Graphics Capture",
    "Desktop Duplication",
    "Desktop GDI",
    "Client GDI",
];

pub fn has_content(pixels: &[u8]) -> bool {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .any(|p| p[..3].iter().any(|c| *c > 80))
}

pub fn first_usable<T>(
    mut capture: impl FnMut(&str) -> Result<T, String>,
    visible: impl Fn(&T) -> bool,
    check: impl Fn() -> Result<(), String>,
) -> Result<(T, &'static str), String> {
    let mut failures = Vec::new();
    for backend in BACKENDS {
        check()?;
        let image = capture(backend);
        check()?;
        match image {
            Ok(image) if visible(&image) => return Ok((image, backend)),
            Ok(_) => failures.push(format!("{backend}: blank frame")),
            Err(error) => failures.push(format!("{backend}: {error}")),
        }
    }
    Err(format!(
        "No usable image from the cloud window. {}",
        failures.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blank_success_falls_through_to_a_readable_frame() {
        let mut tried = Vec::new();
        let (frame, backend) = first_usable(
            |b| {
                tried.push(b.to_string());
                Ok(if b == "Client GDI" {
                    vec![200, 100, 90, 255]
                } else {
                    vec![0, 0, 0, 255]
                })
            },
            |p: &Vec<u8>| has_content(p),
            || Ok(()),
        )
        .unwrap();
        assert_eq!(backend, "Client GDI");
        assert!(has_content(&frame));
        assert_eq!(tried.len(), 4);
    }
    #[test]
    fn exhausted_backends_report_errors_and_blank_frames_separately() {
        let error = first_usable(
            |b| {
                if b == BACKENDS[0] {
                    Err("unsupported".into())
                } else {
                    Ok(vec![0, 0, 0, 255])
                }
            },
            |p: &Vec<u8>| has_content(p),
            || Ok(()),
        )
        .unwrap_err();
        assert!(error.contains("Windows Graphics Capture: unsupported"));
        assert!(error.contains("Desktop Duplication: blank frame"));
        assert!(error.contains("Client GDI: blank frame"));
    }
    #[test]
    fn focus_loss_cancels_instead_of_trying_another_backend() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let error = first_usable(
            |_| {
                calls.set(calls.get() + 1);
                Ok(false)
            },
            |p| *p,
            || {
                if calls.get() > 0 {
                    Err("cancelled".into())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
        assert_eq!(error, "cancelled");
        assert_eq!(calls.get(), 1);
    }
    #[test]
    fn alpha_does_not_make_black_pixels_readable() {
        assert!(!has_content(&[0, 0, 0, 255, 0, 0, 0, 255]));
        assert!(has_content(&[120, 100, 150, 255]));
        assert!(!has_content(&[]));
    }
}
