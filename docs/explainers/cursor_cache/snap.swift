import Cocoa
import WebKit

// usage: swift snap.swift <file.html> <outdir> <width> <height> <dark|light> <sectionId,...>
let a = CommandLine.arguments
let url = URL(fileURLWithPath: a[1]); let outDir = a[2]
let w = CGFloat(Double(a[3])!), h = CGFloat(Double(a[4])!)
let dark = a[5] == "dark"
let ids = a[6].split(separator: ",").map(String.init)

let app = NSApplication.shared
let cfg = WKWebViewConfiguration()
let collector = WKUserScript(source: "window.__errs=[];window.addEventListener('error',e=>window.__errs.push(String(e.message)+' @'+(e.filename||'')+':'+e.lineno));window.addEventListener('unhandledrejection',e=>window.__errs.push('rejection: '+String(e.reason)));", injectionTime: .atDocumentStart, forMainFrameOnly: true)
cfg.userContentController.addUserScript(collector)
let wv = WKWebView(frame: NSRect(x: 0, y: 0, width: w, height: h), configuration: cfg)
wv.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
final class Nav: NSObject, WKNavigationDelegate { var done = false; func webView(_ v: WKWebView, didFinish n: WKNavigation!) { done = true } }
let nav = Nav(); wv.navigationDelegate = nav
let win = NSWindow(contentRect: wv.frame, styleMask: [.borderless], backing: .buffered, defer: false)
win.contentView = wv
wv.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
func spin(_ s: Double) { RunLoop.main.run(until: Date(timeIntervalSinceNow: s)) }
while !nav.done { spin(0.05) }
spin(1.5)

var i = 0
func shoot() {
  if i >= ids.count {
    wv.evaluateJavaScript("JSON.stringify(window.__errs)") { r, _ in print("JS errors: \(String(describing: r))"); exit(0) }
    while true { spin(0.1) }
  }
  let id = ids[i]; i += 1
  let js = id == "top" ? "window.scrollTo(0,0); window.scrollY" : "var el=document.getElementById('\(id)'); window.scrollTo(0, el.getBoundingClientRect().top + window.scrollY - 8); window.scrollY"
  wv.evaluateJavaScript(js) { y, err in
    print("scrollY for \(id): \(String(describing: y)) \(err.map { String(describing: $0) } ?? "")")
    spin(1.2)
    wv.takeSnapshot(with: nil) { img, err in
      if let img = img, let tiff = img.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff), let png = rep.representation(using: .png, properties: [:]) {
        try! png.write(to: URL(fileURLWithPath: "\(outDir)/\(a[5])-\(id).png"))
        print("wrote \(id)")
      } else { print("snapshot failed for \(id): \(String(describing: err))") }
      shoot()
    }
  }
}
shoot()
while true { spin(0.1) }
