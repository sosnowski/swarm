fn main() {
    let arr: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0];

    let sum: f32 = arr.iter().fold(0.0, |acc, item| {
        return acc + item;
    });
    let avg: f32 = sum / (arr.len() as f32);
    println!("Normal AVG: {} / {} = {}", sum, arr.len(), avg);

    let agg_avg: f32 = arr.iter().enumerate().fold(0.0, |avg, (index, &item)| {
        return avg + ((item - avg) / (index as f32 + 1.0));
    });

    println!("Aggregated AVG: {}", agg_avg);
}
