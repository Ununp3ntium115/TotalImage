<script lang="ts">
  import Dashboard from './lib/Dashboard.svelte';
  import FileBrowser from './lib/FileBrowser.svelte';
  import './app.css';
</script>

<main>
  <Dashboard />
  <FileBrowser />

  <footer>
    <p>
      TotalImage - Open Source Forensic Disk Image Analysis
      | <a href="https://github.com/Ununp3ntium115/TotalImage" target="_blank">GitHub</a>
    </p>
  </footer>
</main>

<style>
  main {
    min-height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    padding: 2rem 0;
  }

  footer {
    text-align: center;
    padding: 2rem;
    color: white;
    margin-top: 3rem;
  }

  footer a {
    color: white;
    text-decoration: underline;
  }

  footer a:hover {
    opacity: 0.8;
  }
</style>
