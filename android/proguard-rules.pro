# JNA uses reflection to load native methods
-keep class com.sun.jna.** { *; }
-keep class trafilatura.** implements com.sun.jna.Callback { *; }

# UniFFI-generated code uses JNA Native.register() with reflection
-keep class trafilatura.UniffiLib { *; }
-keep class trafilatura.IntegrityCheckingUniffiLib { *; }
-keep class trafilatura.RustBuffer { *; }
-keep class trafilatura.RustBuffer$* { *; }
-keep class trafilatura.ForeignBytes { *; }
-keep class trafilatura.ForeignBytes$* { *; }
