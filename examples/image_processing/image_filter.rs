use rustray::task::{Task, TaskResult};
use rustray::head::HeadNode;
use image::{ImageBuffer, Rgb};
use std::error::Error;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 初始化头节点
    let head = HeadNode::new("127.0.0.1".to_string(), 8000);
    
    // 加载原始图像
    let img = image::open("input.jpg")?;
    let (width, height) = img.dimensions();
    
    // 将图像分成多个块进行处理
    let block_height = height / 4; // 将图像分成4块
    let mut tasks = Vec::new();
    
    for y in (0..height).step_by(block_height as usize) {
        // 创建图像块处理任务
        let task = Task {
            name: "apply_filter".to_string(),
            args: vec![
                // 序列化图像块数据
                serialize_image_block(&img, 0, y, width, block_height)?,
                // 滤镜参数
                "gaussian_blur".to_string(),
                "3.0".to_string(), // sigma值
            ],
            priority: 0,
        };
        tasks.push(task);
    }
    
    // 提交任务到分布式系统
    println!("Processing image with size {}x{}", width, height);
    let mut results = Vec::new();
    for task in tasks {
        let task_id = head.submit_task(task).await?;
        results.push(task_id);
    }
    
    // 收集处理结果并合并
    let mut final_image = ImageBuffer::new(width, height);
    let mut current_y = 0;
    
    for task_id in results {
        if let Some(result) = head.get_task_result(task_id).await? {
            let block: ImageBuffer<Rgb<u8>, Vec<u8>> = bincode::deserialize(&result.data)?;
            copy_image_block(&mut final_image, &block, 0, current_y);
            current_y += block_height;
        }
    }
    
    // 保存处理后的图像
    final_image.save("output.jpg")?;
    println!("Image processing completed!");
    Ok(())
}

fn serialize_image_block(
    img: &image::DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let block = img.crop(x, y, width, height);
    Ok(bincode::serialize(&block)?)
}

fn copy_image_block(
    dst: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    src: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    x: u32,
    y: u32,
) {
    for (sx, sy, pixel) in src.enumerate_pixels() {
        dst.put_pixel(x + sx, y + sy, *pixel);
    }
}

// Worker节点上运行的图像处理函数
#[cfg(test)]
mod tests {
    use image::{ImageBuffer, Rgb};
    
    fn apply_gaussian_blur(
        img: &ImageBuffer<Rgb<u8>, Vec<u8>>,
        sigma: f32,
    ) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
        // 实现高斯模糊算法
        // 这里省略具体实现
        img.clone()
    }
} 