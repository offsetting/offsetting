use klask::Settings;
use offsetting::Offsetting;

fn main() {
  klask::run_derived::<Offsetting, _>(Settings::default(), |o| {
    if let Err(err) = o.execute() {
      eprintln!("Error: {:#?}", err);
    }
  });
}
