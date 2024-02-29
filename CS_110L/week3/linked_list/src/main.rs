pub mod linked_list;

use linked_list::{ComputeNorm, LinkedList};

fn main() {
    // ALL the Test code is from : https://github.com/fung-hwang/CS110L-2020spr/blob/main/week3/linked_list/src/main.rs
    let mut list: LinkedList<f64> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    let vec_str = vec![1.0, 2.0, 3.0];
    for s in vec_str {
        list.push_front(s.clone());
    }
    println!("list = {}", list);

    // test Clone
    let list_2 = list.clone();

    // test PartialEq
    println!("list == list_2: {}", list == list_2);

    // test ComputeNorm
    println!("compute_norm(list) = {}", list.compute_norm());

    // test Iterator and IntoIterator
    for val in &list {
        println!("{}", val);
    }

    for val in list {
        println!("{}", val);
    }

    // code down there should be error
    // println!("{}",list);
    
}