import "./App.css";

function App() {
  return (
    <main className="app-shell">
      <header className="topbar">
        <a className="brand" href="#home" aria-label="System Scanner home">
          <span className="brand-mark" aria-hidden="true">
            <span />
            <span />
            <span />
            <span />
          </span>
          <span className="brand-name">system<span>scanner</span></span>
        </a>
        <span className="build-label"><span className="status-dot" /> FIRST BUILD</span>
      </header>

      <section className="welcome" aria-labelledby="welcome-title">
        <div className="eyebrow"><span className="eyebrow-line" /> YOUR SYSTEM, IN FOCUS</div>
        <div className="welcome-icon" aria-hidden="true">
          <div className="icon-ring ring-outer" />
          <div className="icon-ring ring-inner" />
          <div className="icon-core"><span /><span /><span /></div>
          <span className="icon-spark spark-one" />
          <span className="icon-spark spark-two" />
        </div>
        <h1 id="welcome-title">Hello, world<span className="title-period">.</span></h1>
        <p className="welcome-copy">
          A clearer view of your PC starts here.
          <br />
          This is the first step for System Scanner.
        </p>
        <div className="welcome-divider"><span /><span /><span /></div>
        <p className="welcome-note">Your system is ready to be explored.</p>
      </section>

      <footer className="footer">
        <span>SYSTEM SCANNER <span className="footer-separator">/</span> WINDOWS</span>
        <span className="version">PREVIEW <span>0.1.0</span></span>
      </footer>
    </main>
  );
}

export default App;
