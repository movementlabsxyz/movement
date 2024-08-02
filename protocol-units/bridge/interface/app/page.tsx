import Image from "next/image";
import Container from "./components/Container";

const footer = [
  {
    title: "Docs",
    href: "https://docs.movementlabs.xyz",
    logo: "/docs.svg",
  },
  {
    title: "GitHub",
    href: "https://github.com/movementlabsxyz",
    logo: "/github.svg",
  },
  {
    title: "Discord",
    href: "https://discord.gg/movementlabsxyz",
    logo: "/discord.svg",
  },
  {
    title: "X",
    href: "https://x.com/movementlabsxyz",
    logo: "/x.svg",
  }
]
export default function Home() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-24">
      <div className=" w-full max-w-5xl items-center justify-between mono text-sm lg:flex">
        Movement Bridge
      </div>

      <div className=" flex place-items-center before:absolute before:h-[300px] before:w-full before:-translate-x-1/2 before:rounded-full before:blur-2xl before:content-[''] after:absolute  after:h-[180px] after:w-full after:translate-x-1/3 after:content-[''] sm:before:w-[700px] sm:after:w-[480px] before:lg:h-[360px]">
       <Container />
      </div>

      <div className="mb-32 grid text-center lg:mb-0 lg:w-full lg:max-w-xl lg:grid-cols-4 lg:text-left">
        {footer.map((item) => (
          <a
            key={item.title}
          href={item.href}
          className="group text-center rounded-lg border border-transparent px-5 py-4 transition-colors hover:border-gray-300 hover:bg-gray-100 hover:dark:border-neutral-700 hover:dark:bg-neutral-800/30"
          target="_blank"
          rel="noopener noreferrer"
        >
          {item.title}
        </a>))}
      </div>
    </main>
  );
}
