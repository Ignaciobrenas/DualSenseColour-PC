import iconUrl from './assets/icon.svg';

export function App() {
  return (
    <main className="flex h-screen w-screen flex-col items-center justify-center rounded-[22px] border border-white/12 bg-[#0a0c12]/80 p-6 text-slate-100 backdrop-blur-2xl">
      <img src={iconUrl} alt="DualSenseColour PC" className="mb-4 h-20 w-20" />
      <h1 className="text-xl font-semibold tracking-tight">DualSenseColour PC</h1>
      <p className="mt-1 text-sm text-slate-400">
        Control nativo de barra LED DualSense en PC
      </p>
    </main>
  );
}

export default App;
