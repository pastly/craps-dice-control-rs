mod randroll;
mod roll;
use roll::Roll;

fn main() {
    let r = Roll::new([7, 2]);
    println!("{:?}", r);
}
