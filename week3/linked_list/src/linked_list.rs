use std::fmt;
use std::option::Option;
pub struct LinkedList<T> {
    head: Option<Box<Node<T>>>,
    size: usize,
}

pub struct LinkedListIter<'a,T> {
    current: &'a Option<Box<Node<T>>>,
}

struct Node <T>{
    value: T,
    next: Option<Box<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(value: T, next: Option<Box<Node<T>>>) -> Node<T> {
        Node {value: value, next: next}
    }
}

impl<T> LinkedList <T>{
    pub fn new() -> LinkedList<T> {
        LinkedList {head: None, size: 0}
    }
    
    pub fn get_size(&self) -> usize {
        self.size
    }
    
    pub fn is_empty(&self) -> bool {
        self.get_size() == 0
    }
    
    pub fn push_front(&mut self, value: T) {
        let new_node: Box<Node<T>> = Box::new(Node::new(value, self.head.take()));
        self.head = Some(new_node);
        self.size += 1;
    }
    
    pub fn pop_front(&mut self) -> Option<T> {
        let node: Box<Node<T>> = self.head.take()?;
        self.head = node.next;
        self.size -= 1;
        Some(node.value)
    }
}


impl<T:fmt::Display> fmt::Display for LinkedList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut current: &Option<Box<Node<T>>> = &self.head;
        let mut result = String::new();
        loop {
            match current {
                Some(node) => {
                    result = format!("{} {}", result, node.value);
                    current = &node.next;
                },
                None => break,
            }
        }
        write!(f, "{}", result)
    }
}

impl<T> Drop for LinkedList<T> {
    fn drop(&mut self) {
        let mut current = self.head.take();
        while let Some(mut node) = current {
            current = node.next.take();
        }
    }
}

impl <T:Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node {
            value:self.value.clone(),
            next:self.next.clone(),
        }
    } 
}

impl<T:PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.next == other.next
    } 
}

impl <T:PartialEq> PartialEq for LinkedList<T> {
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && self.size == other.size
    } 
}

#[allow(unused_doc_comments)]
impl<T:Clone> Clone for LinkedList<T>{
    fn clone(&self) -> Self {
        /// It might seem a little bit troublesome

        // let mut new_list = LinkedList::new();
        // let mut current = &self.head;
        // loop {
        //     match current {
        //         Some(node) => {
        //             new_list.push_front(node.value.clone());
        //             current = &node.next;
        //         },
        //         None => break,
        //     }
        // }
        //  new_list

        /// all parts of LinkedList are impl clone,It can just iteration 
        /// once impl Node Clone trait
        LinkedList {
            head:self.head.clone(),
            size:self.size.clone(),
        }
       
    }
}

impl <T> Iterator for LinkedList<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.pop_front()
    }
}


impl<T:Clone> Iterator for LinkedListIter<'_,T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self.current {
            Some(node) => {
                // YOU FILL THIS IN!
                self.current = &node.next;
                Some(node.value.clone())
            },
            None => // YOU FILL THIS IN!
                None
        }
    }
}

impl<'a,T:Clone> IntoIterator for &'a LinkedList<T>{
    type Item = T;
    type IntoIter = LinkedListIter<'a,T>;
    fn into_iter(self) -> LinkedListIter<'a,T> {
        LinkedListIter {current: &self.head}
    }
}

pub trait ComputeNorm {
    fn compute_norm(&self) -> f64 {
        0.0
    }
}

impl ComputeNorm for LinkedList<f64> {
    fn compute_norm(&self) -> f64 {
        let mut res:f64 = 0.0;
        for iter in self.into_iter(){
            res += iter*iter;
        }
        res.sqrt()
    }
}