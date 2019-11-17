mod randroll;
mod roll;
mod buffer;
use roll::Roll;

fn buf_test() {
    use std::io::{self, Read};
    use buffer::CommentStripIter;
    let mut csi = CommentStripIter::new(io::stdin());
    let mut s = String::new();
    csi.read_to_string(&mut s);
    print!("{}", s);
}

fn main() {
    //let r = Roll::new([1, 2]);
    //println!("{:?}", r);
    buf_test();
}
