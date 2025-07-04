/// Embedded patches for sql-language-server
pub const COMPLETE_JS_PATCH: &str = r#"--- a/complete.js
+++ b/complete.js
@@ -310,11 +310,11 @@
     }
 }
 function complete(sql, pos, schema = { tables: [], functions: [] }, jupyterLabMode = false) {
-    console.time('complete');
+    // console.time('complete');  // Disabled to prevent stdout pollution
     if (logger.isDebugEnabled())
         logger.debug(`complete: ${sql}, ${JSON.stringify(pos)}`);
     const completer = new Completer(schema, sql, pos, jupyterLabMode);
     const candidates = completer.complete();
-    console.timeEnd('complete');
+    // console.timeEnd('complete');  // Disabled to prevent stdout pollution
     return { candidates: candidates, error: completer.error };
 }"#;

pub const INITIALIZE_LOGGING_JS_PATCH: &str = r#"--- a/initializeLogging.js
+++ b/initializeLogging.js
@@ -40,7 +40,7 @@
         },
         // TODO: Should accept level
         categories: {
-            default: { appenders: ['server'], level: debug ? 'debug' : 'debug' },
+            default: { appenders: ['server'], level: debug ? 'debug' : 'error' },
         },
     });
     const logger = log4js_1.default.getLogger();"#;

pub const SETTING_STORE_JS_PATCH: &str = r#"--- a/SettingStore.js
+++ b/SettingStore.js
@@ -74,6 +74,11 @@
         return this.personalConfig;
     }
     async changeConnection(connectionName) {
+        // PATCH: Add safety check
+        if (!this.personalConfig || !this.personalConfig.connections || !Array.isArray(this.personalConfig.connections)) {
+            logger.error(`Cannot change connection - invalid personalConfig: ${JSON.stringify(this.personalConfig)}`);
+            throw new Error('Invalid personal config structure');
+        }
         const config = this.personalConfig.connections.find((v) => v.name === connectionName);
         if (!config) {
             const errorMessage = `not find connection name: ${connectionName}`;
@@ -85,6 +90,7 @@
         let personalConfig = { connections: [] }, projectConfig = {};
         if (fileExists(personalConfigPath)) {
             personalConfig = JSON.parse(readFile(personalConfigPath));
+            logger.debug(`Loaded personal config from ${personalConfigPath}: ${JSON.stringify(personalConfig)}`);
             this.personalConfig = personalConfig;
             logger.debug(`Found personalConfig. ${JSON.stringify(personalConfig)}`);
         }
@@ -93,6 +99,13 @@
         }
         if (fileExists(projectConfigPath)) {
             projectConfig = JSON.parse(readFile(projectConfigPath));
+            logger.debug(`Loaded project config from ${projectConfigPath}: ${JSON.stringify(projectConfig)}`);
+            // PATCH: If projectConfig has connections array, it's likely the full config
+            if (projectConfig.connections && Array.isArray(projectConfig.connections)) {
+                logger.debug('Project config has connections array, treating as personal config');
+                this.personalConfig = projectConfig;
+                personalConfig = projectConfig;
+            }
         }
         else {
             logger.debug(`There isn't project config file., ${projectConfigPath}`);
@@ -146,6 +159,11 @@
         this.emit('change', this.state);
     }
     extractPersonalConfigMatchedProjectPath(projectPath) {
+        // PATCH: Add safety check
+        if (!this.personalConfig || !this.personalConfig.connections || !Array.isArray(this.personalConfig.connections)) {
+            logger.error(`Invalid personalConfig structure: ${JSON.stringify(this.personalConfig)}`);
+            return null;
+        }
         const con = this.personalConfig.connections.find((v) => { var _a; return (_a = v.projectPaths) === null || _a === void 0 ? void 0 : _a.includes(projectPath); });
         if (!con) {
             logger.debug(`Not found personal config, { path: ${projectPath} }`);"#;