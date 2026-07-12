use image::DynamicImage;
use pixie_core::{CoreError, ExportOptions, ImageFormat, ImageOperation};
use std::sync::{Arc, Mutex};
use std::thread;

pub enum WorkerOutput {
  Progress(f32),
  Decoded {
    img: DynamicImage,
    format: ImageFormat,
  },
  OperationApplied {
    img: DynamicImage,
  },
  Exported {
    path: std::path::PathBuf,
  },
  Error(CoreError),
}

pub struct ImageWorker {
  generation: Arc<Mutex<u64>>,
  sender: async_channel::Sender<(u64, WorkerOutput)>,
}

impl ImageWorker {
  pub fn new(sender: async_channel::Sender<(u64, WorkerOutput)>) -> Self {
    Self {
      generation: Arc::new(Mutex::new(0)),
      sender,
    }
  }

  pub fn cancel_current_tasks(&self) -> u64 {
    let mut generation = self.generation.lock().unwrap();
    *generation += 1;
    *generation
  }

  pub fn spawn_decode(&self, data: Vec<u8>) -> u64 {
    let task_gen = self.cancel_current_tasks();
    let sender = self.sender.clone();
    let generation_clone = self.generation.clone();

    thread::spawn(move || {
      // Check if cancelled early
      if *generation_clone.lock().unwrap() != task_gen {
        return;
      }

      let _ = sender.send_blocking((task_gen, WorkerOutput::Progress(0.1)));
      match pixie_core::decode_image(&data) {
        Ok((img, format)) => {
          if *generation_clone.lock().unwrap() == task_gen {
            let _ = sender.send_blocking((task_gen, WorkerOutput::Decoded { img, format }));
          }
        }
        Err(err) => {
          if *generation_clone.lock().unwrap() == task_gen {
            let _ = sender.send_blocking((task_gen, WorkerOutput::Error(err)));
          }
        }
      }
    });

    task_gen
  }

  pub fn spawn_apply_operation(&self, img: DynamicImage, op: ImageOperation) -> u64 {
    let task_gen = self.cancel_current_tasks();
    let sender = self.sender.clone();
    let generation_clone = self.generation.clone();

    thread::spawn(move || {
      if *generation_clone.lock().unwrap() != task_gen {
        return;
      }

      let _ = sender.send_blocking((task_gen, WorkerOutput::Progress(0.2)));
      match op.apply(img) {
        Ok(out_img) => {
          if *generation_clone.lock().unwrap() == task_gen {
            let _ =
              sender.send_blocking((task_gen, WorkerOutput::OperationApplied { img: out_img }));
          }
        }
        Err(err) => {
          if *generation_clone.lock().unwrap() == task_gen {
            let _ = sender.send_blocking((task_gen, WorkerOutput::Error(err)));
          }
        }
      }
    });

    task_gen
  }

  pub fn spawn_export(
    &self,
    img: DynamicImage,
    file_path: std::path::PathBuf,
    original_path: Option<std::path::PathBuf>,
    metadata_policy: pixie_core::MetadataPolicy,
    options: ExportOptions,
  ) -> u64 {
    let task_gen = self.cancel_current_tasks();
    let sender = self.sender.clone();
    let generation_clone = self.generation.clone();

    thread::spawn(move || {
      if *generation_clone.lock().unwrap() != task_gen {
        return;
      }

      let _ = sender.send_blocking((task_gen, WorkerOutput::Progress(0.1)));

      // Atomic export:
      // 1. Create a secure temporary file in the destination directory
      let parent_dir = file_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
      let temp_res = tempfile::NamedTempFile::new_in(parent_dir);

      match temp_res {
        Ok(mut temp_file) => {
          let _ = sender.send_blocking((task_gen, WorkerOutput::Progress(0.4)));

          // Encode to the temp file
          match pixie_core::encode_image(&img, &mut temp_file, &options) {
            Ok(()) => {
              let _ = sender.send_blocking((task_gen, WorkerOutput::Progress(0.8)));

              // Apply metadata policy before finalizing!
              if let Some(ref orig_p) = original_path {
                let temp_path = temp_file.path();
                if let Err(e) =
                  pixie_core::clean_image_metadata(orig_p, temp_path, &metadata_policy)
                {
                  tracing::error!("Metadata cleaning failed: {}", e);
                }
              }

              // Flush and sync
              if let Ok(()) = temp_file.as_file_mut().sync_all() {
                // Persist/rename
                match temp_file.persist(&file_path) {
                  Ok(_) => {
                    if *generation_clone.lock().unwrap() == task_gen {
                      let _ = sender
                        .send_blocking((task_gen, WorkerOutput::Exported { path: file_path }));
                    }
                  }
                  Err(e) => {
                    if *generation_clone.lock().unwrap() == task_gen {
                      let _ = sender.send_blocking((
                        task_gen,
                        WorkerOutput::Error(CoreError::Encode(format!(
                          "Failed to rename temp file to destination: {e}"
                        ))),
                      ));
                    }
                  }
                }
              } else {
                if *generation_clone.lock().unwrap() == task_gen {
                  let _ = sender.send_blocking((
                    task_gen,
                    WorkerOutput::Error(CoreError::Encode(
                      "Failed to synchronize temporary file".into(),
                    )),
                  ));
                }
              }
            }
            Err(err) => {
              if *generation_clone.lock().unwrap() == task_gen {
                let _ = sender.send_blocking((task_gen, WorkerOutput::Error(err)));
              }
            }
          }
        }
        Err(err) => {
          if *generation_clone.lock().unwrap() == task_gen {
            let _ = sender.send_blocking((
              task_gen,
              WorkerOutput::Error(CoreError::Encode(format!(
                "Failed to create temporary file in target directory: {err}"
              ))),
            ));
          }
        }
      }
    });

    task_gen
  }
}
