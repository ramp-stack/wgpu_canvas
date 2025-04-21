+    fn _get_path(name: &str, #[cfg(target_os = "android")] app: &AndroidApp) -> PathBuf {+        #[cfg(target_os="android")]
+        {
+            // PathBuf::from(env!("HOME")).join(format!(".{name}"))
+            app.internal_data_path().unwrap().join(format!(".{name}"))
+        }
+
        #+    pub fn get_path(name: &str, #[cfg(target_os = "android")] app: &AndroidApp) -> PathBuf {
            +        let path = Self::_get_path(name, #[cfg(target_os = "android")] app);
              
        +    pub(crate) async fn new(name: &str, #[cfg(target_os = "android")] app: &AndroidApp) -> Self {
            +        let path = AppStorage::get_path(name, #[cfg(target_os = "android")] app).join("cache.db");
                     let db = rusqlite::Connection::open(path).unwrap();
                     db.execute(