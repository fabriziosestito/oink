import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useBaseUrl from '@docusaurus/useBaseUrl';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import Heading from '@theme/Heading';

import styles from './index.module.css';

const FEATURES = [
  {
    title: 'Stories that branch',
    body: 'Choices, consequences, and several endings. A decision can come back chapters later.',
  },
  {
    title: '2d6 skill checks',
    body: 'Roll two dice, add an ability and any modifiers, and beat the difficulty. Double six always wins, double one always loses.',
  },
  {
    title: 'Your world, your rules',
    body: 'Define abilities, perks, conditions, and items in plain YAML. The engine tracks them and uses them in checks.',
  },
  {
    title: 'Built for e-ink',
    body: 'The same engine runs on Linux, Windows, and macOS today. An M5Paper firmware and phones come next.',
  },
];

const ROADMAP = [
  'M5Paper firmware (ESP32 with an IT8951 e-ink panel)',
  'Prose, dialog, and map layouts chosen by story tags',
  'Bookmarks: save and load your place in the story',
  'iOS and Android builds',
];

function HomepageHero() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={styles.hero}>
      <div className={clsx('container', styles.heroInner)}>
        <div className={styles.heroArt}>
          <img
            src={useBaseUrl('/img/oink-logo-dither.png')}
            className={clsx(styles.logo, styles.logoDither)}
            alt=""
            width={300}
            height={312}
          />
          <img
            src={useBaseUrl('/img/oink-logo.png')}
            className={clsx(styles.logo, styles.logoColor)}
            alt="A pig knight reading a large red book"
            width={300}
          />
        </div>
        <div className={styles.heroCopy}>
          <p className={styles.kicker}>oink</p>
          <Heading as="h1" className={styles.title}>
            Gamebooks for e-ink
          </Heading>
          <p className={styles.lede}>
            {siteConfig.tagline}. oink brings branching stories, 2d6 skill
            checks, and data-driven rules to screens that stay readable in
            sunlight. Think choose-your-own-adventure, with dice.
          </p>
          <div className={styles.buttons}>
            <Link className="button button--primary button--lg" to="/docs">
              Read the docs
            </Link>
            <Link
              className="button button--outline button--lg"
              href="https://github.com/fabriziosestito/oink">
              GitHub
            </Link>
          </div>
          <p className={styles.status}>
            oink is in early development. The desktop simulator runs today;
            e-ink firmware is next.
          </p>
        </div>
      </div>
    </header>
  );
}

function FeatureCard({title, body}) {
  return (
    <article className={styles.feature}>
      <h3 className={styles.featureTitle}>{title}</h3>
      <p className={styles.featureBody}>{body}</p>
    </article>
  );
}

export default function Home() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout
      title="Gamebooks for e-ink"
      description="oink is a gamebook engine for e-ink devices. Branching stories, 2d6 skill checks, and data-driven rules defined in Ink and YAML.">
      <HomepageHero />
      <main className="container">
        <section className={styles.features}>
          {FEATURES.map((feature) => (
            <FeatureCard key={feature.title} {...feature} />
          ))}
        </section>
        <section className={styles.roadmap}>
          <p className={styles.roadmapLabel}>Roadmap</p>
          <h2 className={styles.roadmapTitle}>Coming soon</h2>
          <ul className={styles.roadmapList}>
            {ROADMAP.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
          <Link
            className={styles.roadmapLink}
            href="https://github.com/fabriziosestito/oink/issues">
            Follow the roadmap on GitHub
          </Link>
        </section>
      </main>
    </Layout>
  );
}
