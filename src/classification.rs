//! Evaluation of image classification
//!
//! Provides abstractions for reading image classification output
//! , reading image classification dataset groundtruth and evaluating
//! single-class and multi-class classification techniques.

use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::option::Option;

use log;

use crate::errors;
use crate::utils;
use crate::utils::{ToOneHot, TopK};
use num_traits::ToPrimitive;
/// Generic struct to store the image classification output for a number of images.
pub struct ClassificationOutput<
    T1: num_traits::PrimInt + num_traits::Unsigned + num_traits::FromPrimitive,
    T2: num_traits::Float + fast_float::FastFloat + num_traits::FromPrimitive,
> {
    num_classes: T1,
    data: HashMap<String, Vec<T2>>,
}

impl<
        T1: num_traits::PrimInt + num_traits::Unsigned + num_traits::FromPrimitive,
        T2: num_traits::Float + fast_float::FastFloat + num_traits::FromPrimitive,
    > ClassificationOutput<T1, T2>
{
    /// Creates a new empty instance of [`Self`]
    ///
    /// Items need to be subsequently added to it using [`Self::add()`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// let cls_out = ClassificationOutput::<u8, f64>::new(20u8);
    /// assert_eq!(cls_out.num_classes(), 20u8);
    /// assert_eq!(cls_out.is_empty(), true);
    /// ```
    ///
    /// ```rust
    /// // The following creates a new empty instance of ClassificationOutput<usize,f32> with     ///
    /// //  1000 classes.
    /// use bagheera::classification::ClassificationOutput;
    /// let mut cls_output = ClassificationOutput::<usize, f32>::new(1000usize);
    /// assert_eq!(cls_output.num_classes(), 1000usize);
    /// assert_eq!(cls_output.is_empty(), true);
    /// ```
    pub fn new(num_classes: T1) -> Self {
        ClassificationOutput {
            num_classes,
            data: HashMap::<String, Vec<T2>>::new(),
        }
    }

    /// Adds a new entry to a [`Self`] instance.
    ///
    /// This returns an [`io::Error`] instance if the new entry has different number of classes
    /// than that of the [`Self`] instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// let mut cls_out = ClassificationOutput::new(10usize);
    /// let images = vec!["india.jpg", "germany.png", "iran.jpg"];
    /// for img in images{
    ///     let v = vec![1f64; 10];
    ///     cls_out.add(img, v);
    /// }
    /// assert_eq!(cls_out.num_images(), 3usize);
    /// ```
    pub fn add(&mut self, image_name: &str, confidence_vector: Vec<T2>) -> Result<(), io::Error> {
        if confidence_vector.len() == T1::to_usize(&self.num_classes).unwrap() {
            &self.data.insert(image_name.to_string(), confidence_vector);
            log::debug!("Added record to ClassificationOutput.");
            Ok(())
        } else {
            Err(errors::image_not_present_error(image_name))
        }
    }

    /// Creates a new instance of [`Self`] from a CSV file.
    pub fn from_csv_file(csv_filename: &str, num_classes: T1) -> Result<Self, io::Error> {
        let fid = utils::open_file(csv_filename).unwrap();
        let mut bufread = BufReader::new(fid);
        let mut numlines = 0usize;
        for _ in bufread.by_ref().lines() {
            numlines += 1;
        }
        log::debug!(
            "There are a total of {} lines in {}.",
            numlines,
            csv_filename
        );
        let mut data_hmap = HashMap::<String, Vec<T2>>::with_capacity(numlines);
        bufread.seek(SeekFrom::Start(0u64)).unwrap();
        log::debug!("Reading and parsing lines from the file.");
        for (line_num, line) in bufread.lines().enumerate() {
            let line = line.unwrap();
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Line {} of {} is empty. This is an error.",
                        line_num, csv_filename
                    ),
                ));
            }
            let mut imagename = String::new();

            for (token_num, token) in line_trimmed.split(",").enumerate() {
                if token_num == 0usize {
                    imagename = token.to_string();
                    data_hmap.insert(token.to_string(), Vec::<T2>::new());
                    continue;
                }

                data_hmap
                    .get_mut(&imagename)
                    .unwrap()
                    .push(fast_float::parse::<T2, _>(token).unwrap());
            }
            if T1::from_usize(data_hmap[&imagename].len()).unwrap() != num_classes {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Line {} of {} contains {} classes when num_classes is specified as {}",
                        line_num,
                        csv_filename,
                        data_hmap[&imagename].len(),
                        num_classes.to_usize().unwrap()
                    ),
                ));
            }
        }
        log::debug!("Finished parsing the file.");
        Ok(ClassificationOutput {
            num_classes,
            data: data_hmap,
        })
    }
    /// Returns the number of object classes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    ///  let cls_out = ClassificationOutput::<u32, f64>::new(20u32);
    ///  assert_eq!(cls_out.num_classes(), 20u32);
    /// ```
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    ///  let cls_out = ClassificationOutput::<usize, f32>::new(1000usize);
    ///  assert_eq!(cls_out.num_classes(), 1000usize);
    /// ```
    #[inline(always)]
    pub fn num_classes(&self) -> T1 {
        self.num_classes
    }

    /// Returns the number of images in a [`Self`] instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// let mut cls_out = ClassificationOutput::<usize,f64>::new(10usize);
    /// let images = vec!["india.jpg", "germany.png", "iran.jpg"];
    /// for img in images{
    ///     let v = vec![1f64; 10];
    ///     cls_out.add(img, v);
    /// }
    /// assert_eq!(cls_out.num_images(), 3usize);
    /// ```
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// let mut cls_out = ClassificationOutput::<u8, f32>::new(30u8);
    /// let images = vec!["india.jpg", "germany.png", "iran.jpg", "canada.png", "japan.jpg"];
    /// for img in images{
    ///     let v = vec![1f32; 30];
    ///     cls_out.add(img, v);
    /// }
    /// assert_eq!(cls_out.num_images(), 5usize);
    /// ```
    #[inline(always)]
    pub fn num_images(&self) -> usize {
        self.data.len()
    }

    /// Returns true if `image_name` is present in a [`Self`] instance. False otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// let mut cls_out = ClassificationOutput::<u8, f32>::new(30u8);
    /// let images = vec!["india.jpg", "germany.png", "iran.jpg", "canada.png", "japan.jpg"];
    /// for img in images{
    ///     let v = vec![1f32; 30];
    ///     cls_out.add(img, v);
    /// }
    /// assert_eq!(cls_out.image_is_present("australia.jpg"), false);
    /// ```
    #[inline(always)]
    pub fn image_is_present(&self, image_name: &str) -> bool {
        self.data.contains_key(image_name)
    }

    /// Returns a vector of image names in a [`Self`] instance.
    ///
    /// The returned vector contains `&str` slices to the `String` stored in the instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// use std::collections::HashSet;
    /// let mut cls_out = ClassificationOutput::<u8, f32>::new(30u8);
    /// let mut images = vec!["india.jpg", "germany.png", "iran.jpg", "canada.png", "japan.jpg"];
    /// images.sort();
    /// for img in &images{
    ///     let v = vec![1f32; 30];
    ///     cls_out.add(img, v);
    /// }
    /// let list_of_images = cls_out.list_images();
    /// let mut lhs = HashSet::<&str>::with_capacity(list_of_images.len());
    /// for item in list_of_images{
    ///     lhs.insert(item);
    /// }
    ///
    /// let mut rhs = HashSet::<&str>::with_capacity(images.len());
    /// for item in images{
    ///     rhs.insert(item);
    /// }
    /// assert_eq!(lhs, rhs);
    /// ```
    pub fn list_images(&self) -> Vec<&str> {
        self.data.iter().map(|it| it.0.as_str()).collect()
    }

    /// Returns true if a [`Self`] instance is empty. False otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationOutput;
    /// let cls_out = ClassificationOutput::<u8, f64>::new(20u8);
    /// assert_eq!(cls_out.num_classes(), 20u8);
    /// assert_eq!(cls_out.is_empty(), true);
    /// ```
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the confidence vector for `imagename` if it exists in the [`Self`] instance.
    /// An [io::Error] instance is returned otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    ///     use bagheera::classification::ClassificationOutput;
    ///     use rand;
    ///     use rand::Rng;
    ///     use float_cmp::approx_eq;
    ///    let mut cls_out = ClassificationOutput::<u32,f32>::new(1000u32);
    ///     let rand_iter = rand::thread_rng().gen_range(0usize..500usize);
    ///     println!("{}",rand_iter);
    ///     let mut test_image = String::new();
    ///     let mut test_vec = Vec::<f32>::new();
    ///    for i in 0usize..500usize{
    ///         let rand_name : String = (0..50).map(|_| rand::random::<u8>() as char).collect();
    ///         let rand_vec = vec![rand::random::<f32>() ; 1000usize];
    ///         if i == rand_iter{
    ///             test_vec = rand_vec.clone();
    ///             test_image = rand_name.clone();
    ///             println!("Test image : {}",test_image);
    ///         }
    ///         cls_out.add(&rand_name, rand_vec);
    ///     }
    ///
    /// for (lhs, rhs) in (*(cls_out.confidence_for_image(&test_image).unwrap())).iter().zip(test_vec.iter()){
    ///     approx_eq!(f32, *lhs, *rhs, ulps=5);
    /// }
    /// ```
    #[inline(always)]
    pub fn confidence_for_image(&self, imagename: &str) -> Result<&Vec<T2>, io::Error> {
        if !self.image_is_present(imagename) {
            return Err(errors::image_not_present_error(imagename));
        } else {
            Ok(&self.data[imagename])
        }
    }

    /// Returns indices for the Top-k entries of the confidence vector for an image in
    /// a [`Self`] instance.
    ///
    /// If `imagename` is not in the [`Self`] instance, an [io::Error] instance is returned
    ///
    /// # Examples
    ///
    /// ```rust
    ///     use bagheera::classification::ClassificationOutput;
    ///     use rand;
    ///     use rand::Rng;
    ///     use float_cmp::approx_eq;
    ///    let mut cls_out = ClassificationOutput::<u32,f32>::new(1000u32);
    ///     let rand_iter = rand::thread_rng().gen_range(0usize..500usize);
    ///     let mut test_image = String::new();
    ///     let mut test_vec = Vec::<f32>::new();
    ///    for i in 0usize..500usize{
    ///         let rand_name : String = (0..50).map(|_| rand::random::<u8>() as char).collect();
    ///         let mut rand_vec = Vec::<f32>::new();
    ///         if i == rand_iter{
    ///             test_image = rand_name.clone();
    ///             rand_vec = (0..1000).map(|x| x as f32).collect();
    ///         }
    ///         else {
    ///             rand_vec =  vec![rand::random::<f32>() ; 1000usize];
    ///         }
    ///         cls_out.add(&rand_name, rand_vec);
    ///     }
    ///
    ///     let topk_ind = cls_out.topk_for_image(&test_image,3usize).unwrap();
    ///     assert_eq!(topk_ind, vec![999usize, 998usize, 997usize])
    ///```
    pub fn topk_for_image(&self, imagename: &str, k: usize) -> Result<Vec<usize>, io::Error>
    where
        Vec<T2>: utils::TopK,
    {
        if !self.image_is_present(imagename) {
            return Err(errors::image_not_present_error(imagename));
        }
        let topk_indices = (*self.confidence_for_image(imagename).unwrap())
            .top_k(k)
            .unwrap();

        Ok(topk_indices)
    }
}

/// Generic struct representing an image classification dataset.
pub struct ClassificationDataset<
    T1: num_traits::PrimInt + num_traits::Unsigned + num_traits::FromPrimitive,
> {
    num_classes: T1,
    data: HashMap<String, Vec<bool>>,
    is_multilabel: bool,
}

impl<T1: num_traits::PrimInt + num_traits::Unsigned + num_traits::FromPrimitive>
    ClassificationDataset<T1>
{
    /// Returns a new empty instance of [`Self<T1>`].
    ///
    /// Images can be added to the instance using the [`Self::add()`] function.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///
    /// let cls_db = ClassificationDataset::new(30u8, false);
    /// assert_eq!(cls_db.num_classes(), 30u8);
    /// ```
    pub fn new(num_classes: T1, is_multilabel: bool) -> Self {
        ClassificationDataset {
            num_classes,
            data: HashMap::<String, Vec<bool>>::new(),
            is_multilabel,
        }
    }

    /// Adds a new GT to the [`Self`] instance.
    ///
    /// If `imagename` is already in [`Self`] instance, an [io::Error] instance is returned.
    /// In case of issues during one-hot conversion, a panic happens.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///
    /// let mut cls_db = ClassificationDataset::new(5u16, false);
    /// cls_db.add("hello.jpg", &vec![1u16]);
    /// assert_eq!(cls_db.num_images(), 1usize);
    /// ```
    pub fn add(&mut self, imagename: &str, category_labels: &Vec<T1>) -> Result<(), io::Error> {
        if self.image_is_present(imagename) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Image {} was already present.", imagename),
            ));
        }
        if !self.is_multilabel && category_labels.len() > 1 {
            return Err(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Tried adding multi-label data to a ClassificationDataset instance that is not multilabel.",
                )
            );
        }
        self.data.insert(
            imagename.to_string(),
            category_labels.convert(self.num_classes()),
        );
        Ok(())
    }
    /// Returns the number of object classes in the [`Self`] instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///  let cls_db = ClassificationDataset::<u32>::new(20u32, false);
    ///  assert_eq!(cls_db.num_classes(), 20u32);
    /// ```
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///  let cls_db = ClassificationDataset::<u128>::new(2000u128, true);
    ///  assert_eq!(cls_db.num_classes(), 2000u128);
    /// ```
    #[inline(always)]
    pub fn num_classes(&self) -> T1 {
        self.num_classes
    }

    #[inline(always)]
    pub fn num_images(&self) -> usize {
        self.data.len()
    }

    #[inline(always)]
    pub fn image_is_present(&self, imagename: &str) -> bool {
        self.data.contains_key(imagename)
    }

    #[inline(always)]
    pub fn list_images(&self) -> Vec<&str> {
        self.data.iter().map(|it| it.0.as_str()).collect()
    }

    /// Returns true if the [`Self`] instance represents a multi-label classification dataset.
    /// Otherwise false is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///
    /// let cls_db = ClassificationDataset::new(30u16, true);
    /// assert_eq!(cls_db.is_multilabel(), true);
    /// ```
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///
    /// let cls_db = ClassificationDataset::new(1000u16, false);
    /// assert_eq!(cls_db.is_multilabel(), false);
    /// ```
    #[inline(always)]
    pub fn is_multilabel(&self) -> bool {
        self.is_multilabel
    }
    /// Gets the groundtruth for `imagename` in [`Self`] instance.
    ///
    /// If `imagename` is not in the [`Self`] instance, an [io::Error]
    /// instance is returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///
    /// let mut cls_db = ClassificationDataset::new(5u16, false);
    /// cls_db.add("hello.jpg", &vec![1u16]);
    /// assert_eq!(cls_db.get_gt("hello.jpg").unwrap(), &vec![false, true, false, false, false]);
    /// ```
    ///
    /// ```rust
    /// use bagheera::classification::ClassificationDataset;
    ///
    /// let mut cls_db = ClassificationDataset::new(5u16, true);
    /// cls_db.add("hello.jpg", &vec![1u16, 3u16]);
    /// assert_eq!(cls_db.get_gt("hello.jpg").unwrap(), &vec![false, true, false, true, false]);
    /// ```
    #[inline]
    pub fn get_gt(&self, imagename: &str) -> Result<&Vec<bool>, io::Error> {
        if !self.image_is_present(imagename) {
            Err(errors::image_not_present_error(imagename))
        } else {
            Ok(&self.data[imagename])
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

pub trait EvaluationOptions {
    fn is_multilabel(&self) -> bool;
    fn gen_default() -> Self;
    fn list_options(&self) -> &[&str];
}

macro_rules! create_with_field_names {
    (pub struct $name:ident { $($fname:ident : $ftype:ty),* }) => {
         pub struct $name {
            $($fname : $ftype),*
        }

        impl $name {
            pub fn field_names() -> &'static [&'static str] {
                static NAMES: &'static [&'static str] = &[$(stringify!($fname)),*];
                NAMES
            }
        }
    }
}

create_with_field_names! {
    pub struct SingleLabelEvaluationOptions{
        k_values : Vec<usize>,
        per_class_analysis : bool
    }
}

create_with_field_names! {
    pub struct MultipleLabelEvaluationOptions{
        per_class_analysis : bool
    }
}

impl EvaluationOptions for SingleLabelEvaluationOptions {
    fn is_multilabel(&self) -> bool {
        false
    }

    fn gen_default() -> Self {
        SingleLabelEvaluationOptions {
            k_values: vec![1usize, 5usize, 10usize],
            per_class_analysis: true,
        }
    }

    fn list_options(&self) -> &[&str] {
        SingleLabelEvaluationOptions::field_names()
    }
}

impl SingleLabelEvaluationOptions {
    fn new(&self, k_values: &Vec<usize>, per_class_analysis: bool) -> Self {
        SingleLabelEvaluationOptions {
            k_values: k_values.to_vec(),
            per_class_analysis,
        }
    }
}

impl EvaluationOptions for MultipleLabelEvaluationOptions {
    fn is_multilabel(&self) -> bool {
        true
    }

    fn gen_default() -> Self {
        MultipleLabelEvaluationOptions {
            per_class_analysis: true,
        }
    }

    fn list_options(&self) -> &[&str] {
        MultipleLabelEvaluationOptions::field_names()
    }
}

impl MultipleLabelEvaluationOptions {
    fn new(per_class_analysis: bool) -> Self {
        MultipleLabelEvaluationOptions { per_class_analysis }
    }
}

/// Generic struct abstracting the data available to a judge for classification evaluation.
pub struct ClassificationJudge<
    'a,
    T1: num_traits::PrimInt + num_traits::Unsigned + num_traits::FromPrimitive,
    T2: num_traits::Float + fast_float::FastFloat + num_traits::FromPrimitive,
    T3: EvaluationOptions,
> {
    classifier_output: &'a ClassificationOutput<T1, T2>,
    dataset: &'a ClassificationDataset<T1>,
    evaluation_options: T3,
}

enum DIFFERENCE {
    POSITIVE,
    NEGATIVE,
}

impl<
        'a,
        T1: num_traits::PrimInt + num_traits::Unsigned + num_traits::FromPrimitive,
        T2: num_traits::Float + fast_float::FastFloat + num_traits::FromPrimitive,
        T3: EvaluationOptions,
    > ClassificationJudge<'a, T1, T2, T3>
{
    pub fn new(
        classifier_output: &'a ClassificationOutput<T1, T2>,
        dataset: &'a ClassificationDataset<T1>,
        evaluation_options: T3,
    ) -> Self {
        if !(dataset.is_multilabel() && evaluation_options.is_multilabel()) {
            panic!();
        }

        ClassificationJudge {
            classifier_output,
            dataset,
            evaluation_options,
        }
    }

    #[inline(always)]
    fn get_difference(&self, imagename: &str) -> Vec<DIFFERENCE>
    where
        T2: Copy,
    {
        self.dataset
            .get_gt(imagename)
            .unwrap()
            .iter()
            .map(|x| match *x {
                true => DIFFERENCE::NEGATIVE,
                _ => DIFFERENCE::POSITIVE,
            })
            .collect::<Vec<DIFFERENCE>>()
    }

    pub fn threshold_confidence(&self, conf: &Vec<T2>, threshold: T2) -> Vec<T2> {
        conf.iter()
            .map(|x| match *x > threshold {
                true => *x,
                _ => T2::zero(),
            })
            .collect::<Vec<T2>>()
    }
}

pub struct ClassificationResult<
    T2: num_traits::Float + fast_float::FastFloat + num_traits::FromPrimitive,
> {
    topk_accuracy: HashMap<usize, T2>,
    per_class_accuracy: Option<Vec<T2>>,
    per_class_precision: Option<Vec<T2>>,
    per_class_recall: Option<Vec<T2>>,
    per_class_ap: Option<Vec<T2>>,
    per_class_f1: Option<Vec<T2>>,
}
